use crate::device::{Device, DeviceShared};
use crate::error::Error;
use crate::instance::InstanceShared;
use ash::vk::{DeviceMemory, ExternalMemoryHandleTypeFlags, ImportMemoryFdInfoKHR, MemoryAllocateInfo};
use std::ffi::c_void;

#[derive(Clone, Copy, Debug)]
pub struct MemoryTypeIndex(u32);
impl MemoryTypeIndex {
    pub fn new(type_index: u32) -> Self {
        Self(type_index)
    }
}

pub(crate) struct AllocationShared<'a> {
    shared_instance: &'a InstanceShared,
    shared_device: &'a DeviceShared<'a>,
    device_memory: DeviceMemory,
    // size: u64,
    // type_index: MemoryTypeIndex,
}

impl<'a> AllocationShared<'a> {
    pub fn new(shared_device: &'a DeviceShared<'a>, size: u64, type_index: MemoryTypeIndex) -> Result<Self, Error> {
        let native_device = shared_device.native();
        let info = MemoryAllocateInfo::default().allocation_size(size).memory_type_index(type_index.0);
        let device_memory = unsafe { native_device.allocate_memory(&info, None)? };

        Ok(Self {
            shared_instance: shared_device.instance(),
            shared_device,
            device_memory,
            // size,
            // type_index,
        })
    }

    pub fn new_external(shared_device: &'a DeviceShared<'a>, external: *mut c_void, size: u64) -> Result<Self, Error> {
        let native_device = shared_device.native();

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
                shared_instance: shared_device.instance(),
                shared_device,
                device_memory,
                // size,
                // type_index: MemoryTypeIndex(0), // TODO
            })
        }
    }

    #[expect(unused)]
    pub(crate) fn instance(&self) -> &InstanceShared {
        &self.shared_instance
    }

    pub(crate) fn device(&self) -> &DeviceShared {
        &self.shared_device
    }

    pub(crate) fn native(&self) -> DeviceMemory {
        self.device_memory
    }
}

impl<'a> Drop for AllocationShared<'a> {
    fn drop(&mut self) {
        let native_device = self.shared_device.native();

        unsafe {
            native_device.free_memory(self.device_memory, None);
        }
    }
}

/// An allocation on a host or device.
pub struct Allocation<'a> {
    shared: AllocationShared<'a>,
}

impl<'a> Allocation<'a> {
    pub fn new(device: &'a Device, size: u64, type_index: MemoryTypeIndex) -> Result<Self, Error> {
        let allocation_shared = AllocationShared::new(device.shared(), size, type_index)?;

        Ok(Self { shared: allocation_shared })
    }

    pub fn new_external(device: &'a Device, external: *mut c_void, size: u64) -> Result<Self, Error> {
        let allocation_shared = AllocationShared::new_external(device.shared(), external, size)?;

        Ok(Self { shared: allocation_shared })
    }

    pub(crate) fn shared(&self) -> &AllocationShared {
        &self.shared
    }

    pub(crate) fn native(&self) -> DeviceMemory {
        self.shared.native()
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
