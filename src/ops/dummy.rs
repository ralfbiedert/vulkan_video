use crate::error::Error;
use crate::ops::AddToCommandBuffer;
use crate::queue::CommandBuilder;

/// NOP operation.
pub struct Dummy {}

impl Dummy {
    pub fn new() -> Self {
        Self {}
    }
}

impl AddToCommandBuffer for Dummy {
    fn run_in(&self, _: &mut CommandBuilder) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::commandbuffer::CommandBuffer;
    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::ops::{AddToCommandBuffer, Dummy};
    use crate::physicaldevice::PhysicalDevice;
    use crate::queue::Queue;
    #[test]
    #[cfg(not(miri))]
    fn create_queue() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let compute_queue = physical_device.queue_family_infos().any_compute().ok_or(Error::QueueNotFound)?;
        let device = Device::new(&physical_device)?;
        let queue = Queue::new(&device, compute_queue, 0)?;
        let command_buffer = CommandBuffer::new(&device, compute_queue)?;
        let dummy = Dummy::new();

        queue.build_and_submit(&command_buffer, |x| {
            dummy.run_in(x)?;
            Ok(())
        })?;

        Ok(())
    }
}
