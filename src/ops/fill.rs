use crate::error::Error;
use crate::ops::AddToCommandBuffer;
use crate::queue::CommandBuilder;
use crate::resources::{Buffer, BufferShared};
use ash::vk;
use ash::vk::{DependencyFlags, PipelineStageFlags, WHOLE_SIZE};
use std::sync::Arc;

/// Fills a buffer with a fixed value.
pub struct FillBuffer {
    buffer: Arc<BufferShared>,
    value: u32,
}

impl FillBuffer {
    pub fn new(buffer: &Buffer, value: u32) -> Self {
        Self {
            buffer: buffer.shared(),
            value,
        }
    }
}

impl AddToCommandBuffer for FillBuffer {
    fn run_in(&self, builder: &mut CommandBuilder) -> Result<(), Error> {
        let native_device = self.buffer.device().native();
        let native_buffer = self.buffer.native();
        let native_command_buffer = builder.native_command_buffer();

        // TODO: Do we want to keep these barriers as part of these operations (but then we'd sort
        // of have to divine what the subsequent operations are). Or do we want barriers to be
        // explicit operations (but then people might forget using them or won't use them correctly)?
        let buffer_barrier = vk::BufferMemoryBarrier::default()
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            .buffer(native_buffer)
            .size(self.buffer.size())
            .offset(0);

        let barriers = [buffer_barrier];

        unsafe {
            native_device.cmd_pipeline_barrier(
                native_command_buffer,
                PipelineStageFlags::TRANSFER,
                PipelineStageFlags::TRANSFER,
                DependencyFlags::empty(),
                &[],
                &barriers,
                &[], // No image-level memory barriers
            );

            native_device.cmd_fill_buffer(native_command_buffer, native_buffer, 0, WHOLE_SIZE, self.value);
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
    use crate::ops::{AddToCommandBuffer, FillBuffer};
    use crate::physicaldevice::PhysicalDevice;
    use crate::queue::Queue;
    use crate::resources::{Buffer, BufferInfo};

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
        let allocation = Allocation::new(&device, 1024, host_visible)?;

        let buffer_info = BufferInfo::new().size(1024);
        let buffer = Buffer::new(&allocation, &buffer_info)?;

        let fill_buffer = FillBuffer::new(&buffer, 0x11223344);

        queue.build_and_submit(&command_buffer, |x| {
            fill_buffer.run_in(x)?;
            Ok(())
        })?;

        let mut data = vec![0; 1024];
        buffer.download_into(&mut data)?;

        assert_eq!(data[3], 0x11);
        assert_eq!(data[2], 0x22);
        assert_eq!(data[1], 0x33);
        assert_eq!(data[0], 0x44);

        Ok(())
    }
}
