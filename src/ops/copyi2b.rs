use crate::error::Error;
use crate::ops::AddToCommandBuffer;
use crate::queue::CommandBuilder;
use crate::resources::{Buffer, BufferShared, Image, ImageShared};
use ash::vk::{BufferImageCopy, ImageAspectFlags, ImageLayout, ImageSubresourceLayers};

/// Performs an image-to-buffer copy operation.
pub struct CopyImage2Buffer<'a> {
	image: &'a ImageShared<'a>,
	buffer: &'a BufferShared<'a>,
    aspect_mask: ImageAspectFlags,
}

impl<'a> CopyImage2Buffer<'a> {
    pub fn new(image: &'a Image<'a>, buffer: &'a Buffer<'a>, aspect_mask: ImageAspectFlags) -> Self {
        Self {
            image: image.shared(),
            buffer: buffer.shared(),
            aspect_mask,
        }
    }
}

impl<'a> AddToCommandBuffer for CopyImage2Buffer<'a> {
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

        unsafe {
            native_device.cmd_copy_image_to_buffer(native_command_buffer, native_image, ImageLayout::GENERAL, native_buffer, &[copy]);
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
    use crate::resources::{Buffer, BufferInfo, Image, ImageInfo};
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
        let image = Image::new(&device, &image_info)?;
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
