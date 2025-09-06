use crate::allocation::MemoryTypeIndex;
use crate::error;
use crate::error::{Error, Variant};
use crate::instance::{Instance, InstanceShared};
use ash::vk::{MemoryPropertyFlags, PhysicalDeviceMemoryProperties, QueueFlags};

/// Provides logical information about vulkan queue families.
pub struct QueueFamilyInfos {
    queue_compute: Option<u32>,
    queue_decode: Option<u32>,
    available_queues: Vec<u32>,
}

impl QueueFamilyInfos {
    unsafe fn new(instance: ash::Instance, physical_device: ash::vk::PhysicalDevice) -> Self {
        unsafe {
            let queue_family_properties = instance.get_physical_device_queue_family_properties(physical_device);

            let queue_compute = queue_family_properties
                .iter()
                .enumerate()
                .find(|x| x.1.queue_flags.contains(QueueFlags::COMPUTE))
                .map(|x| x.0 as u32);

            let queue_decode = queue_family_properties
                .iter()
                .enumerate()
                .find(|x| x.1.queue_flags.contains(QueueFlags::VIDEO_DECODE_KHR))
                .map(|x| x.0 as u32);

            let mut available_queues = Vec::with_capacity(2);

            if let Some(x) = queue_compute {
                available_queues.push(x)
            }

            if let Some(x) = queue_decode {
                available_queues.push(x)
            }

            Self {
                queue_compute,
                queue_decode,
                available_queues,
            }
        }
    }
    pub fn available(&self) -> &[u32] {
        &self.available_queues
    }

    pub fn any_compute(&self) -> Option<u32> {
        self.queue_compute
    }

    pub fn any_decode(&self) -> Option<u32> {
        self.queue_decode
    }
}

/// Provides logical information about Vulkan memory heaps.
pub struct HeapInfos {
    memory_properties: PhysicalDeviceMemoryProperties,
}

impl HeapInfos {
    unsafe fn new(instance: ash::Instance, physical_device: ash::vk::PhysicalDevice) -> Self {
        unsafe {
            let memory_properties = instance.get_physical_device_memory_properties(physical_device);

            Self { memory_properties }
        }
    }

    pub fn any_host_visible(&self) -> Option<MemoryTypeIndex> {
        for i in 0..self.memory_properties.memory_type_count as usize {
            let memory_type = self.memory_properties.memory_types[i];

            if memory_type.property_flags.contains(MemoryPropertyFlags::HOST_VISIBLE) {
                return Some(MemoryTypeIndex::new(i as u32));
            }
        }

        None
    }

    pub fn any_device_local(&self) -> Option<MemoryTypeIndex> {
        for i in 0..self.memory_properties.memory_type_count as usize {
            let memory_type = self.memory_properties.memory_types[i];

            if memory_type.property_flags.contains(MemoryPropertyFlags::DEVICE_LOCAL) {
                return Some(MemoryTypeIndex::new(i as u32));
            }
        }

        None
    }
}

pub(crate) struct PhysicalDeviceShared<'a> {
    native_physical_device: ash::vk::PhysicalDevice,
    shared_instance: &'a InstanceShared,
    queue_family_infos: QueueFamilyInfos,
    heap_infos: HeapInfos,
}

impl<'a> PhysicalDeviceShared<'a> {
    pub fn new_any(shared_instance: &'a InstanceShared) -> Result<Self, Error> {
        let native_instance = shared_instance.native();

        unsafe {
            // SAFETY: Should be safe as native instance is valid.
            let mut physical_devices = native_instance.enumerate_physical_devices()?;
            let native_physical_device = physical_devices.pop().ok_or_else(|| error!(Variant::NoVideoDevice))?;
            let queue_family_infos = QueueFamilyInfos::new(native_instance.clone(), native_physical_device);
            let heap_infos = HeapInfos::new(native_instance.clone(), native_physical_device);

            Ok(Self {
                native_physical_device,
                shared_instance,
                queue_family_infos,
                heap_infos,
            })
        }
    }

    pub(crate) fn native(&self) -> ash::vk::PhysicalDevice {
        self.native_physical_device
    }

    pub(crate) fn instance(&self) -> &InstanceShared {
        &self.shared_instance
    }

    pub fn queue_family_infos(&self) -> &QueueFamilyInfos {
        &self.queue_family_infos
    }

    pub fn heap_infos(&self) -> &HeapInfos {
        &self.heap_infos
    }
}

/// Some GPU in your system.
pub struct PhysicalDevice<'a> {
    shared: PhysicalDeviceShared<'a>,
}

impl<'a> PhysicalDevice<'a> {
    pub fn new_any(instance: &'a Instance) -> Result<Self, Error> {
        let shared = PhysicalDeviceShared::new_any(instance.shared())?;

        Ok(Self { shared })
    }

    pub(crate) fn shared(&self) -> &PhysicalDeviceShared<'_> {
        &self.shared
    }

    pub fn queue_family_infos(&self) -> &QueueFamilyInfos {
        self.shared.queue_family_infos()
    }
    pub fn heap_infos(&self) -> &HeapInfos {
        self.shared.heap_infos()
    }
}

#[cfg(test)]
mod test {
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;

    #[test]
    #[cfg(not(miri))]
    fn crate_physical_device() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;

        _ = PhysicalDevice::new_any(&instance)?;

        Ok(())
    }

    #[test]
    #[cfg(not(miri))]
    fn get_queue_family_infos() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;

        _ = physical_device.queue_family_infos();

        Ok(())
    }
}
