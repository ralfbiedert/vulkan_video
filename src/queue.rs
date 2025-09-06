use std::marker::PhantomData;

use ash::vk::{CommandBufferBeginInfo, CommandBufferResetFlags, FenceCreateFlags, FenceCreateInfo, SubmitInfo};

use crate::commandbuffer::{CommandBuffer, CommandBufferShared};
use crate::device::{Device, DeviceShared};
use crate::error::Error;

pub struct CommandBuilder<'a> {
    _lt: PhantomData<&'a ()>,
    native_command_buffer: ash::vk::CommandBuffer,
    queue_family_index: u32,
}

impl<'a> CommandBuilder<'a> {
    pub fn native_command_buffer(&self) -> ash::vk::CommandBuffer {
        self.native_command_buffer
    }

    pub fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }
}

struct QueueShared {
    shared_device: DeviceShared,
    native_queue: ash::vk::Queue,
    queue_family_index: u32,
}

impl QueueShared {
    fn new(shared_device: DeviceShared, queue_family_index: u32, index: u32) -> Result<Self, Error> {
        let native_device = shared_device.native();

        unsafe {
            let native_queue = native_device.get_device_queue(queue_family_index, index);

            Ok(Self {
                shared_device,
                native_queue,
                queue_family_index,
            })
        }
    }

    pub fn build_and_submit(
        &self,
        command_buffer: CommandBufferShared,
        f: impl FnOnce(&mut CommandBuilder) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let native_device = self.shared_device.native();
        let native_command_buffer = command_buffer.native();
        let native_queue = self.native_queue;

        let begin_info = CommandBufferBeginInfo::default();
        let command_buffers = [native_command_buffer];
        let submit_info = SubmitInfo::default().command_buffers(&command_buffers);
        let fence_info = FenceCreateInfo::default().flags(FenceCreateFlags::default());

        let mut queue_live = CommandBuilder {
            _lt: Default::default(),
            native_command_buffer,
            queue_family_index: self.queue_family_index,
        };

        unsafe {
            let fence = native_device.create_fence(&fence_info, None)?;

            native_device.reset_command_buffer(native_command_buffer, CommandBufferResetFlags::empty())?;
            native_device.begin_command_buffer(native_command_buffer, &begin_info)?;
            f(&mut queue_live)?;
            native_device.end_command_buffer(native_command_buffer)?;
            // TODO - nevermind, this still about 1 in 5 times fails on this line ... (DEVICE LOST)
            native_device.queue_submit(native_queue, &[submit_info], fence)?;
            native_device.wait_for_fences(&[fence], true, u64::MAX)?;
            native_device.destroy_fence(fence, None);
            native_device.queue_wait_idle(native_queue)?;

            Ok(())
        }
    }
}

/// GPU execution unit to run your command buffers.
pub struct Queue {
    shared: QueueShared,
}

impl Queue {
    pub fn new(device: &Device, family: u32, index: u32) -> Result<Self, Error> {
        let shared = QueueShared::new(device.shared(), family, index)?;

        Ok(Self { shared })
    }

    pub fn build_and_submit(
        &self,
        command_buffer: &CommandBuffer,
        f: impl FnOnce(&mut CommandBuilder) -> Result<(), Error>,
    ) -> Result<(), Error> {
        self.shared.build_and_submit(command_buffer.shared(), f)
    }
}

#[cfg(test)]
mod test {
    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;
    use crate::queue::Queue;

    #[test]
    #[cfg(not(miri))]
    fn create_queue() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;

        _ = Queue::new(&device, 0, 0)?;

        Ok(())
    }
}
