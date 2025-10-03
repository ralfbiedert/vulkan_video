use crate::error::Error;
use crate::ops::AddToCommandBuffer;
use crate::queue::CommandBuilder;
use crate::resources::{Buffer, BufferShared, Image, ImageShared};
use ash::vk::{
    AccessFlags, BufferImageCopy, DependencyFlags, ImageAspectFlags, ImageLayout, ImageMemoryBarrier, ImageSubresourceLayers,
    ImageSubresourceRange, PipelineStageFlags, QUEUE_FAMILY_IGNORED,
};
use std::rc::Rc;
use std::sync::Arc;

/// Performs an image-to-buffer copy operation.
pub struct CopyImage2Buffer {
    image: Rc<ImageShared>,
    buffer: Arc<BufferShared>,
    aspect_mask: ImageAspectFlags,
}

impl CopyImage2Buffer {
    pub fn new(image: &Image, buffer: &Buffer, aspect_mask: ImageAspectFlags) -> Self {
        Self {
            image: image.shared(),
            buffer: buffer.shared(),
            aspect_mask,
        }
    }
}

impl AddToCommandBuffer for CopyImage2Buffer {
    fn run_in(&self, builder: &mut CommandBuilder) -> Result<(), Error> {
        let native_device = self.image.device().native();
        let native_command_buffer = builder.native_command_buffer();
        let native_image = self.image.native();
        let native_buffer = self.buffer.native();

        let image_info = self.image.info();

        let srl = ImageSubresourceLayers::default().aspect_mask(self.aspect_mask).layer_count(1);

        let copy = BufferImageCopy::default()
            .image_extent(image_info.get_extent())
            .image_subresource(srl);

        let ssr = ImageSubresourceRange::default()
            .aspect_mask(ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);

        let barrier_acquire = ImageMemoryBarrier::default()
            .old_layout(ImageLayout::UNDEFINED)
            .new_layout(ImageLayout::GENERAL)
            .image(native_image)
            .subresource_range(ssr)
            .src_access_mask(AccessFlags::empty())
            .dst_access_mask(AccessFlags::TRANSFER_READ)
            .src_queue_family_index(QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(QUEUE_FAMILY_IGNORED);

        let barrier_release = ImageMemoryBarrier::default()
            .old_layout(ImageLayout::GENERAL)
            .new_layout(ImageLayout::GENERAL)
            .image(native_image)
            .subresource_range(ssr)
            .src_access_mask(AccessFlags::TRANSFER_READ)
            .dst_access_mask(AccessFlags::empty())
            .src_queue_family_index(QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(QUEUE_FAMILY_IGNORED);

        unsafe {
            native_device.cmd_pipeline_barrier(
                native_command_buffer,
                PipelineStageFlags::empty(),
                PipelineStageFlags::TRANSFER,
                DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_acquire],
            );
            native_device.cmd_copy_image_to_buffer(native_command_buffer, native_image, ImageLayout::GENERAL, native_buffer, &[copy]);
            native_device.cmd_pipeline_barrier(
                native_command_buffer,
                PipelineStageFlags::TRANSFER,
                PipelineStageFlags::empty(),
                DependencyFlags::empty(),
                &[],
                &[],
                &[barrier_release],
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::allocation::Allocation;
    use crate::commandbuffer::CommandBuffer;
    use crate::device::Device;
    use crate::error;
    use crate::error::{Error, Variant};
    use crate::instance::{Instance, InstanceInfo};
    use crate::ops::{AddToCommandBuffer, CopyImage2Buffer};
    use crate::physicaldevice::PhysicalDevice;
    use crate::queue::Queue;
    use crate::resources::{Buffer, BufferInfo, ImageInfo, UnboundImage};
    use ash::vk::{Extent3D, Format, ImageAspectFlags, ImageLayout, ImageTiling, ImageType, ImageUsageFlags, SampleCountFlags};

    #[test]
    #[cfg(not(miri))]
    fn copy_image_to_buffer() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let compute_queue = physical_device
            .queue_family_infos()
            .any_compute()
            .ok_or_else(|| error!(Variant::QueueNotFound))?;
        let device = Device::new(&physical_device)?;
        let queue = Queue::new(&device, compute_queue, 0)?;
        let command_buffer = CommandBuffer::new(&device, compute_queue)?;
        let image_info = ImageInfo::new()
            .format(Format::R8_UNORM)
            .samples(SampleCountFlags::TYPE_1)
            .usage(ImageUsageFlags::TRANSFER_SRC | ImageUsageFlags::TRANSFER_DST)
            .mip_levels(1)
            .array_layers(1)
            .image_type(ImageType::TYPE_2D)
            .tiling(ImageTiling::OPTIMAL)
            .layout(ImageLayout::UNDEFINED)
            .extent(Extent3D::default().width(512).height(512).depth(1));
        let image = UnboundImage::new(&device, &image_info)?;
        let host_visible = image.memory_requirement().any_heap();
        let allocation = Allocation::new(&device, 1024 * 1024 * 8, host_visible)?;
        let image = image.bind(&allocation)?;
        let buffer_info = BufferInfo::new().size(1024 * 1024).offset(1024 * 1024);
        let buffer = Buffer::new(&allocation, &buffer_info)?;

        let image2buffer = CopyImage2Buffer::new(&image, &buffer, ImageAspectFlags::COLOR);

        queue.build_and_submit(&command_buffer, |x| {
            image2buffer.run_in(x)?;
            Ok(())
        })?;

        Ok(())
    }
}
