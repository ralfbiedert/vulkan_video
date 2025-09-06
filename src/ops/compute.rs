use ash::vk::{
    AccessFlags, BufferMemoryBarrier, DependencyFlags, DescriptorBufferInfo, DescriptorImageInfo, DescriptorPool, DescriptorPoolCreateInfo,
    DescriptorPoolSize, DescriptorSet, DescriptorSetAllocateInfo, DescriptorType, ImageAspectFlags, ImageLayout, ImageMemoryBarrier,
    ImageSubresourceRange, PipelineBindPoint, PipelineStageFlags, WriteDescriptorSet, QUEUE_FAMILY_IGNORED,
};

use crate::error::Error;
use crate::ops::AddToCommandBuffer;
use crate::queue::CommandBuilder;
use crate::shader::{ParameterType, Pipeline, ShaderParameterSet};

/// Run a compute shader.
pub struct Compute<'a, T> {
    pipeline: &'a Pipeline<'a, T>,
    dispatch_groups: (u32, u32, u32),
    native_descriptor_pool: DescriptorPool,
    native_descriptor_sets: Vec<DescriptorSet>,
    params: T,
}

impl<'a, T: ShaderParameterSet> Compute<'a, T> {
    pub fn new(pipeline: &'a Pipeline<T>, params: T, dispatch_groups: (u32, u32, u32)) -> Result<Self, Error> {
        let parameters = pipeline.parameters();
        let native_device = pipeline.device().native();
        let native_descriptor_set_layout = parameters.native_layout();
        let native_descriptor_set_layouts = &[native_descriptor_set_layout];

        let descriptor_pool_storage = DescriptorPoolSize::default().descriptor_count(3).ty(DescriptorType::STORAGE_BUFFER);
        let descriptor_pool_image = DescriptorPoolSize::default().descriptor_count(3).ty(DescriptorType::STORAGE_IMAGE);

        let descriptor_pool_sizes = &[descriptor_pool_storage, descriptor_pool_image];
        let descriptor_pool_create_info = DescriptorPoolCreateInfo::default().pool_sizes(descriptor_pool_sizes).max_sets(1);

        unsafe {
            let descriptor_pool = native_device.create_descriptor_pool(&descriptor_pool_create_info, None)?;

            let descriptor_set_alloc_info = DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(native_descriptor_set_layouts);

            let descriptor_sets = native_device.allocate_descriptor_sets(&descriptor_set_alloc_info)?;

            Ok(Self {
                pipeline,
                dispatch_groups,
                native_descriptor_pool: descriptor_pool,
                native_descriptor_sets: descriptor_sets,
                params,
            })
        }
    }
}

impl<'a, T> Drop for Compute<'a, T> {
    fn drop(&mut self) {
        unsafe {
            let native_device = self.pipeline.device().native();

            native_device.destroy_descriptor_pool(self.native_descriptor_pool, None);
        }
    }
}

impl<'a, T: ShaderParameterSet> AddToCommandBuffer for Compute<'a, T> {
    fn run_in(&self, builder: &mut CommandBuilder) -> Result<(), Error> {
        let native_device = self.pipeline.device().native();
        let native_command_buffer = builder.native_command_buffer();
        let native_pipeline = self.pipeline.native();
        let native_layout = self.pipeline.layout();

        let mut acquire_image = Vec::new();
        let mut acquire_buffer = Vec::new();
        let mut release_buffer = Vec::new();
        let release_image = Vec::new();

        unsafe {
            let descriptor_set = self.native_descriptor_sets[0];
            let bind_point = PipelineBindPoint::COMPUTE;

            for (i, param) in self.params.parameter_types().iter().enumerate() {
                match param {
                    ParameterType::Buffer { native, size } => {
                        let mut write_descriptor_sets = Vec::new();

                        let descriptor_buffer_info = DescriptorBufferInfo::default().buffer(*native).range(*size);
                        let descriptor_buffer_infos = [descriptor_buffer_info];

                        let write_descriptor_set = WriteDescriptorSet::default()
                            .dst_binding(i as u32)
                            .dst_set(descriptor_set)
                            .descriptor_type(DescriptorType::STORAGE_BUFFER)
                            .buffer_info(&descriptor_buffer_infos);

                        write_descriptor_sets.push(write_descriptor_set);

                        let barrier_acquire = BufferMemoryBarrier::default()
                            .size(*size)
                            .buffer(*native)
                            .src_access_mask(AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE)
                            .src_queue_family_index(QUEUE_FAMILY_IGNORED)
                            .dst_access_mask(AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE)
                            .dst_queue_family_index(builder.queue_family_index());

                        let barrier_release = BufferMemoryBarrier::default()
                            .size(*size)
                            .buffer(*native)
                            .src_access_mask(AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE)
                            .src_queue_family_index(builder.queue_family_index())
                            .dst_access_mask(AccessFlags::MEMORY_READ | AccessFlags::MEMORY_WRITE)
                            .dst_queue_family_index(QUEUE_FAMILY_IGNORED);

                        acquire_buffer.push(barrier_acquire);
                        release_buffer.push(barrier_release);

                        native_device.update_descriptor_sets(&write_descriptor_sets, &[]);
                    }
                    ParameterType::ImageView { native_view, native_image } => {
                        let mut write_descriptor_sets = Vec::new();

                        let descriptor_image_info = DescriptorImageInfo::default()
                            .image_view(*native_view)
                            .image_layout(ImageLayout::GENERAL);

                        let descriptor_image_infos = [descriptor_image_info];

                        let write_descriptor_set = WriteDescriptorSet::default()
                            .dst_binding(i as u32)
                            .dst_set(descriptor_set)
                            .descriptor_type(DescriptorType::STORAGE_IMAGE)
                            .image_info(&descriptor_image_infos);

                        write_descriptor_sets.push(write_descriptor_set);

                        native_device.update_descriptor_sets(&write_descriptor_sets, &[]);

                        let ssr = ImageSubresourceRange::default()
                            .aspect_mask(ImageAspectFlags::COLOR)
                            .level_count(1)
                            .layer_count(1);

                        let barrier = ImageMemoryBarrier::default()
                            .old_layout(ImageLayout::UNDEFINED)
                            .new_layout(ImageLayout::GENERAL)
                            .image(*native_image)
                            .subresource_range(ssr)
                            .src_queue_family_index(QUEUE_FAMILY_IGNORED)
                            .dst_queue_family_index(QUEUE_FAMILY_IGNORED);

                        acquire_image.push(barrier);
                    }
                }
            }

            let x = self.dispatch_groups.0;
            let y = self.dispatch_groups.1;
            let z = self.dispatch_groups.2;

            native_device.cmd_bind_pipeline(native_command_buffer, PipelineBindPoint::COMPUTE, native_pipeline);
            native_device.cmd_bind_descriptor_sets(
                native_command_buffer,
                bind_point,
                native_layout,
                0,
                &self.native_descriptor_sets,
                &[],
            );
            native_device.cmd_pipeline_barrier(
                native_command_buffer,
                PipelineStageFlags::ALL_COMMANDS,
                PipelineStageFlags::COMPUTE_SHADER,
                DependencyFlags::empty(),
                &[],
                &acquire_buffer,
                &acquire_image,
            );
            native_device.cmd_dispatch(native_command_buffer, x, y, z);
            native_device.cmd_pipeline_barrier(
                native_command_buffer,
                PipelineStageFlags::ALL_COMMANDS,
                PipelineStageFlags::HOST,
                DependencyFlags::empty(),
                &[],
                &release_buffer,
                &release_image,
            );

            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use ash::vk::{
        Extent3D, Format, ImageAspectFlags, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, ImageViewType, SampleCountFlags,
    };

    use crate::allocation::Allocation;
    use crate::commandbuffer::CommandBuffer;
    use crate::device::Device;
    use crate::error;
    use crate::error::{Error, Variant};
    use crate::instance::{Instance, InstanceInfo};
    use crate::ops::compute::Compute;
    use crate::ops::copyi2b::CopyImage2Buffer;
    use crate::ops::AddToCommandBuffer;
    use crate::physicaldevice::PhysicalDevice;
    use crate::queue::Queue;
    use crate::resources::{Buffer, BufferInfo, ImageInfo, ImageView, ImageViewInfo, UnboundImage};
    use crate::shader::{Parameters, Pipeline, Shader};

    #[test]
    #[cfg(not(miri))]
    #[expect(clippy::erasing_op)]
    fn compute() -> Result<(), Error> {
        const BLOCK_SIZE: u64 = 1024;

        let shader_code = include_bytes!("../../tests/shaders/compiled/hello_world.spv");

        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;

        let host_visible = physical_device
            .heap_infos()
            .any_host_visible()
            .ok_or_else(|| error!(Variant::HeapNotFound))?;
        let allocation = Allocation::new(&device, 4 * BLOCK_SIZE, host_visible)?;
        let buffer0 = Buffer::new(&allocation, &BufferInfo::new().size(BLOCK_SIZE).offset(0 * BLOCK_SIZE))?;
        let buffer1 = Buffer::new(&allocation, &BufferInfo::new().size(BLOCK_SIZE).offset(1 * BLOCK_SIZE))?;
        let buffer2 = Buffer::new(&allocation, &BufferInfo::new().size(BLOCK_SIZE).offset(2 * BLOCK_SIZE))?;
        let compute_queue = physical_device
            .queue_family_infos()
            .any_compute()
            .ok_or_else(|| error!(Variant::QueueNotFound))?;
        let queue = Queue::new(&device, compute_queue, 0)?;
        let parameters = Parameters::new(&device)?;
        let shader = Shader::new(&device, shader_code, "main", &parameters)?;
        let pipeline = Pipeline::new(&device, &shader)?;
        let command_buffer = CommandBuffer::new(&device, compute_queue)?;

        buffer1.upload(&[3u8; BLOCK_SIZE as usize])?;
        buffer2.upload(&[11u8; BLOCK_SIZE as usize])?;

        let compute = Compute::new(&pipeline, (&buffer0, &buffer1, &buffer2), (1, 1, 1))?;

        queue.build_and_submit(&command_buffer, |x| compute.run_in(x))?;

        let mut data_out = [23u8; BLOCK_SIZE as usize];
        buffer0.download_into(&mut data_out)?;

        assert_eq!(data_out[0], 14);
        assert_eq!(data_out[1], 14);
        assert_eq!(data_out[2], 14);
        assert_eq!(data_out[3], 14);

        Ok(())
    }

    #[test]
    #[cfg(not(miri))]
    fn submit_compute_images() -> Result<(), Error> {
        let shader_code = include_bytes!("../../tests/shaders/compiled/image_color.spv");

        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let image_info = ImageInfo::new()
            .format(Format::A8B8G8R8_SNORM_PACK32)
            .samples(SampleCountFlags::TYPE_1)
            .usage(ImageUsageFlags::TRANSFER_SRC | ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::SAMPLED | ImageUsageFlags::STORAGE)
            .mip_levels(1)
            .array_layers(1)
            .image_type(ImageType::TYPE_2D)
            .tiling(ImageTiling::OPTIMAL)
            .layout(ImageLayout::UNDEFINED)
            .extent(Extent3D::default().width(512).height(512).depth(1));
        let image = UnboundImage::new(&device, &image_info)?;

        let heap_image = image.memory_requirement().any_heap();
        let heap_host_visible = physical_device
            .heap_infos()
            .any_host_visible()
            .ok_or_else(|| error!(Variant::HeapNotFound))?;

        let allocation_gpu = Allocation::new(&device, 512 * 512 * 4, heap_image)?;
        let allocation_host_visible = Allocation::new(&device, 512 * 512 * 4, heap_host_visible)?;

        let image = image.bind(&allocation_gpu)?;

        let image_view_info = ImageViewInfo::new()
            .aspect_mask(ImageAspectFlags::COLOR)
            .format(Format::A8B8G8R8_SNORM_PACK32)
            .image_view_type(ImageViewType::TYPE_2D)
            .layer_count(1)
            .level_count(1);
        let image_view = ImageView::new(&image, &image_view_info)?;
        let compute_queue = physical_device
            .queue_family_infos()
            .any_compute()
            .ok_or_else(|| error!(Variant::QueueNotFound))?;
        let queue = Queue::new(&device, compute_queue, 0)?;
        let parameters = Parameters::new(&device)?;
        let shader = Shader::new(&device, shader_code, "main", &parameters)?;
        let pipeline = Pipeline::new(&device, &shader)?;
        let command_buffer = CommandBuffer::new(&device, compute_queue)?;
        let buffer_info = BufferInfo::new().size(512 * 512 * 4);
        let buffer = Buffer::new(&allocation_host_visible, &buffer_info)?;

        let compute = Compute::new(&pipeline, (&image_view,), (16, 16, 1))?;
        let copy = CopyImage2Buffer::new(&image, &buffer, ImageAspectFlags::COLOR);

        // TODO: SOMETHING HERE GOES WRONG
        queue.build_and_submit(&command_buffer, |x| {
            compute.run_in(x)?;
            copy.run_in(x)?;
            Ok(())
        })?;

        let mut data_out = [0u8; 512 * 512 * 4];
        buffer.download_into(&mut data_out)?;

        assert_eq!(data_out[0], 13);
        assert_eq!(data_out[1], 25);
        assert_eq!(data_out[2], 38);
        assert_eq!(data_out[3], 51);

        Ok(())
    }
}
