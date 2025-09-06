use crate::error::Error;
use crate::ops::AddToCommandBuffer;
use crate::queue::CommandBuilder;
use crate::resources::{Buffer, BufferShared};
use ash::vk::BufferCopy;

/// Performs a buffer-to-buffer copy operation.
pub struct CopyBuffer2Buffer<'a> {
    source: &'a BufferShared<'a>,
    destination: &'a BufferShared<'a>,
    size: u64,
}

impl<'a> CopyBuffer2Buffer<'a> {
    pub fn new(source: &'a Buffer<'a>, destination: &'a Buffer<'a>, size: u64) -> Self {
        Self {
            source: source.shared(),
            destination: destination.shared(),
            size,
        }
    }
}

impl<'a> AddToCommandBuffer for CopyBuffer2Buffer<'a> {
    fn run_in(&self, builder: &mut CommandBuilder) -> Result<(), Error> {
        let native_device = self.source.device().native();
        let native_command_buffer = builder.native_command_buffer();
        let native_source = self.source.native();
        let native_destination = self.destination.native();

        let region = BufferCopy::default().size(self.size);
        let regions = [region];

        unsafe {
            native_device.cmd_copy_buffer(native_command_buffer, native_source, native_destination, &regions);
            Ok(())
        }
    }
}

#[cfg(test)]
mod test {
    use crate::allocation::Allocation;
    use crate::commandbuffer::CommandBuffer;
    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::ops::{AddToCommandBuffer, CopyBuffer2Buffer, FillBuffer};
    use crate::physicaldevice::PhysicalDevice;
    use crate::queue::Queue;
    use crate::resources::{Buffer, BufferInfo};
    use crate::{error, Variant};

    #[test]
    #[cfg(not(miri))]
    fn copy_buffers() -> Result<(), Error> {
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
        let host_visible = physical_device
            .heap_infos()
            .any_host_visible()
            .ok_or_else(|| error!(Variant::HeapNotFound))?;
        let allocation = Allocation::new(&device, 2 * 1024, host_visible)?;

        let buffer_info_src = BufferInfo::new().size(1024);
        let buffer_info_dst = BufferInfo::new().size(1024).offset(1024);

        let buffer_src = Buffer::new(&allocation, &buffer_info_src)?;
        let buffer_dst = Buffer::new(&allocation, &buffer_info_dst)?;

        let fill_buffer = FillBuffer::new(&buffer_src, 0x11223344);
        let copy_buffer = CopyBuffer2Buffer::new(&buffer_src, &buffer_dst, 1024);

        queue.build_and_submit(&command_buffer, |x| {
            fill_buffer.run_in(x)?;
            copy_buffer.run_in(x)?;
            Ok(())
        })?;

        let mut data = vec![0; 1024];
        buffer_dst.download_into(&mut data)?;

        assert_eq!(data[3], 0x11);
        assert_eq!(data[2], 0x22);
        assert_eq!(data[1], 0x33);
        assert_eq!(data[0], 0x44);

        Ok(())
    }
}
