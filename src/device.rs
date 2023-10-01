use crate::error::Error;
use crate::instance::InstanceShared;
use crate::physicaldevice::{PhysicalDevice, PhysicalDeviceShared};
use ash::vk::{DeviceCreateInfo, DeviceQueueCreateInfo, PhysicalDeviceFeatures2, PhysicalDeviceSynchronization2Features};
use std::sync::Arc;

/// Returns a queue family index (`.0`) that supports video, and a queue index (`.1`).
///
/// # Safety
///
/// Must be called with a valid instance and physical device.
// unsafe fn video_decode_queue(instance: ash::Instance, physical_device: PhysicalDevice) -> Option<(u32, u32)> {
//     instance
//         .get_physical_device_queue_family_properties(physical_device)
//         .iter()
//         .enumerate()
//         .find(|(_, property)| property.queue_flags.contains(QueueFlags::VIDEO_DECODE_KHR))
//         .map(|(i, _)| (i as u32, 0)) // TODO: Where to get queue index from?
// }

#[allow(unused)]
pub(crate) struct DeviceShared {
    native_device: ash::Device,
    shared_physical_device: Arc<PhysicalDeviceShared>,
}

impl DeviceShared {
    pub(crate) fn new_with_families(shared_physical_device: Arc<PhysicalDeviceShared>, queue_families: &[u32]) -> Result<Self, Error> {
        let native_instance = shared_physical_device.instance().native();

        // SAFETY: Should be safe as native instance is valid.
        let mut physical_devices = unsafe { native_instance.enumerate_physical_devices()? };
        let native_physical_device = physical_devices.pop().ok_or_else(|| Error::NoVideoDevice)?;

        // TODO: ... MAKE THIS PUBLIC AND
        // SAFETY: Should be safe as native instance and physical device are valid.
        // let (queue_family_index, queue_index) =
        //     unsafe { video_decode_queue(native_instance.clone(), native_physical_device).ok_or(Error::NoVideoDevice)? };

        let device_extensions = [
            b"VK_KHR_video_queue\0".as_ptr() as *const _,
            b"VK_KHR_video_decode_queue\0".as_ptr() as *const _,
            b"VK_KHR_video_decode_h264\0".as_ptr() as *const _,
        ];

        let mut create_infos = Vec::new();

        for family in queue_families {
            let create_info = DeviceQueueCreateInfo::default()
                .queue_family_index(*family)
                .queue_priorities(&[1.0]);

            create_infos.push(create_info);
        }

        let mut sync_features = PhysicalDeviceSynchronization2Features::default().synchronization2(true);
        let mut device_features = PhysicalDeviceFeatures2::default().push_next(&mut sync_features);

        let create_info = DeviceCreateInfo::default()
            .queue_create_infos(&create_infos)
            .push_next(&mut device_features)
            .enabled_extension_names(device_extensions.as_slice());

        unsafe {
            let native_device = native_instance.create_device(native_physical_device, &create_info, None)?;

            Ok(Self {
                native_device,
                shared_physical_device,
            })
        }
    }

    pub(crate) fn new(shared_physical_device: Arc<PhysicalDeviceShared>) -> Result<Self, Error> {
        let infos = shared_physical_device.queue_family_infos().available().to_vec();

        Self::new_with_families(shared_physical_device, &infos)
    }

    pub(crate) fn physical_device(&self) -> Arc<PhysicalDeviceShared> {
        self.shared_physical_device.clone()
    }

    pub(crate) fn instance(&self) -> Arc<InstanceShared> {
        self.shared_physical_device.instance()
    }

    pub(crate) fn native(&self) -> ash::Device {
        self.native_device.clone()
    }
}

impl Drop for DeviceShared {
    fn drop(&mut self) {
        unsafe {
            self.native_device.destroy_device(None);
        }
    }
}

/// Logical Vulkan device linked to some [`PhysicalDevice`](PhysicalDevice).
pub struct Device {
    shared: Arc<DeviceShared>,
}

impl Device {
    pub fn new_with_families(physical_device: &PhysicalDevice, queue_families: &[u32]) -> Result<Self, Error> {
        let device_shared = DeviceShared::new_with_families(physical_device.shared(), queue_families)?;

        Ok(Self {
            shared: Arc::new(device_shared),
        })
    }

    pub fn new(physical_device: &PhysicalDevice) -> Result<Self, Error> {
        let device_shared = DeviceShared::new(physical_device.shared())?;

        Ok(Self {
            shared: Arc::new(device_shared),
        })
    }

    pub(crate) fn shared(&self) -> Arc<DeviceShared> {
        self.shared.clone()
    }
}

#[cfg(test)]
mod test {
    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;

    #[test]
    #[cfg(not(miri))]
    fn crate_device() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;

        _ = physical_device.queue_family_infos();
        _ = Device::new(&physical_device)?;

        Ok(())
    }
}
