use crate::device::Device;
use crate::error;
use crate::error::{Error, Variant};
use ash::vk::{CommandBufferAllocateInfo, CommandBufferLevel, CommandPoolCreateFlags, CommandPoolCreateInfo};

#[allow(unused)]
pub(crate) struct CommandBufferShared<'a> {
    device: &'a Device<'a>,
    native_command_pool: ash::vk::CommandPool,
    native_command_buffer: ash::vk::CommandBuffer,
}

impl<'a> CommandBufferShared<'a> {
    pub fn new(device: &'a Device<'a>, queue_family_index: u32) -> Result<Self, Error> {
        let native_device = device.native();

        let command_pool_create_info = CommandPoolCreateInfo::default()
            .flags(CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family_index);

        unsafe {
            let native_command_pool = native_device.create_command_pool(&command_pool_create_info, None)?;

            let command_buffer_alloc_info = CommandBufferAllocateInfo::default()
                .command_pool(native_command_pool)
                .command_buffer_count(1)
                .level(CommandBufferLevel::PRIMARY);

            let native_command_buffer = native_device
                .allocate_command_buffers(&command_buffer_alloc_info)?
                .pop()
                .ok_or_else(|| error!(Variant::NoCommandBuffer))?;

            Ok(Self {
                device,
                native_command_pool,
                native_command_buffer,
            })
        }
    }

    pub(crate) fn native(&self) -> ash::vk::CommandBuffer {
        self.native_command_buffer
    }
}

impl<'a> Drop for CommandBufferShared<'a> {
    fn drop(&mut self) {
        let device = self.device.native();

        unsafe {
            device.free_command_buffers(self.native_command_pool, &[self.native_command_buffer]);
            device.destroy_command_pool(self.native_command_pool, None);
        }
    }
}

/// Stores commands related to a specific queue family.
#[allow(unused)]
pub struct CommandBuffer<'a> {
    shared: CommandBufferShared<'a>,
}

impl<'a> CommandBuffer<'a> {
    pub fn new(device: &'a Device, queue_family_index: u32) -> Result<Self, Error> {
        let shared = CommandBufferShared::new(device, queue_family_index)?;

        Ok(Self { shared })
    }

    #[allow(unused)]
    pub(crate) fn native(&self) -> ash::vk::CommandBuffer {
        self.shared.native()
    }
}

#[cfg(test)]
mod test {
    use crate::commandbuffer::CommandBuffer;
    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;

    #[test]
    #[cfg(not(miri))]
    fn create_command_pool() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;

        _ = CommandBuffer::new(&device, 0)?;

        Ok(())
    }
}
