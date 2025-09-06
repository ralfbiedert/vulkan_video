use crate::device::Device;
use crate::error::Error;
use crate::instance::Instance;
use ash::vk::{DeviceMemory, ExternalMemoryHandleTypeFlags, ImportMemoryFdInfoKHR, MemoryAllocateInfo};
use std::ffi::c_void;

#[derive(Clone, Copy, Debug)]
pub struct MemoryTypeIndex(u32);
impl MemoryTypeIndex {
    pub fn new(type_index: u32) -> Self {
        Self(type_index)
    }
}

/// An allocation on a host or device.
pub struct Allocation<'a> {
    instance: &'a Instance,
    device: &'a Device<'a>,
    device_memory: DeviceMemory,
    // size: u64,
    // type_index: MemoryTypeIndex,
}

impl<'a> Allocation<'a> {
    pub fn new(device: &'a Device<'a>, size: u64, type_index: MemoryTypeIndex) -> Result<Self, Error> {
        let native_device = device.native();
        let info = MemoryAllocateInfo::default().allocation_size(size).memory_type_index(type_index.0);
        let device_memory = unsafe { native_device.allocate_memory(&info, None)? };

        Ok(Self {
            instance: device.instance(),
            device,
            device_memory,
            // size,
            // type_index,
        })
    }

    pub fn new_external(device: &'a Device<'a>, external: *mut c_void, size: u64) -> Result<Self, Error> {
        let native_device = device.native();

        let mut todo_bad = ImportMemoryFdInfoKHR::default()
            .handle_type(ExternalMemoryHandleTypeFlags::OPAQUE_WIN32) // TODO
            .fd(external as _);

        let info = MemoryAllocateInfo::default()
            .allocation_size(size)
            .memory_type_index(3) // TODO!!
            .push_next(&mut todo_bad);

        unsafe {
            let device_memory = native_device.allocate_memory(&info, None)?;

            Ok(Self {
                instance: device.instance(),
                device,
                device_memory,
                // size,
                // type_index: MemoryTypeIndex(0), // TODO
            })
        }
    }

    #[allow(unused)]
    pub(crate) fn instance(&self) -> &Instance {
        &self.instance
    }

    pub(crate) fn device(&self) -> &Device<'_> {
        &self.device
    }

    pub(crate) fn native(&self) -> DeviceMemory {
        self.device_memory
    }
}

impl<'a> Drop for Allocation<'a> {
    fn drop(&mut self) {
        let native_device = self.device.native();

        unsafe {
            native_device.free_memory(self.device_memory, None);
        }
    }
}

#[cfg(test)]
mod test {
    use crate::allocation::Allocation;
    use crate::device::Device;
    use crate::error;
    use crate::error::{Error, Variant};
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;

    #[test]
    #[cfg(not(miri))]
    fn allocate() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let host_visible = physical_device
            .heap_infos()
            .any_host_visible()
            .ok_or_else(|| error!(Variant::HeapNotFound))?;

        _ = Allocation::new(&device, 16 * 1024, host_visible)?;

        Ok(())
    }
}
