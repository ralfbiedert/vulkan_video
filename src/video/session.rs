use crate::allocation::Allocation;
use crate::device::{Device, DeviceShared};
use crate::error::Error;
use crate::video::h264::H264StreamInspector;
use ash::vk;
use ash::vk::{
    BindVideoSessionMemoryInfoKHR, ExtensionProperties, Extent2D, Format, KhrVideoDecodeQueueFn, KhrVideoQueueFn,
    VideoSessionCreateFlagsKHR, VideoSessionCreateInfoKHR, VideoSessionKHR, VideoSessionMemoryRequirementsKHR,
};
use std::ffi::c_char;
use std::iter::zip;
use std::ptr::null;
use std::sync::Arc;

fn extension_name(name: &[u8]) -> [c_char; 256] {
    let mut extension_name = [0; 256];

    for (y, x) in zip(&mut extension_name, name) {
        *y = *x as c_char;
    }

    extension_name
}

pub(crate) struct VideoSessionShared {
    shared_device: Arc<DeviceShared>,
    native_queue_fns: KhrVideoQueueFn,
    native_decode_queue_fns: KhrVideoDecodeQueueFn,
    native_session: VideoSessionKHR,
    allocations: Vec<Allocation>,
}

impl VideoSessionShared {
    pub fn new(device: &Device, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let shared_device = device.shared();
        let shared_instance = shared_device.instance();

        let native_device = shared_device.native();
        let native_instance = shared_instance.native();
        let native_entry = shared_instance.native_entry();

        let extension_name = extension_name(b"VK_STD_vulkan_video_codec_h264_decode");
        let extension_version = vk::make_api_version(0, 1, 0, 0);

        let extensions_names = ExtensionProperties::default()
            .spec_version(extension_version)
            .extension_name(extension_name);

        let profiles = stream_inspector.profiles();

        let video_session_create_info = VideoSessionCreateInfoKHR::default()
            .queue_family_index(3)
            .flags(VideoSessionCreateFlagsKHR::empty())
            .video_profile(&profiles.info)
            .picture_format(Format::G8_B8R8_2PLANE_420_UNORM)
            .max_coded_extent(Extent2D { width: 512, height: 512 })
            .reference_picture_format(Format::G8_B8R8_2PLANE_420_UNORM)
            .max_dpb_slots(17)
            .max_active_reference_pictures(16)
            .std_header_version(&extensions_names);

        unsafe {
            let queue_fns = KhrVideoQueueFn::load(
                |x| {
                    native_entry
                        .get_instance_proc_addr(native_instance.handle(), x.as_ptr() as *const _)
                        .expect("Must have function pointer") as *const _
                }, // TODO: Is this guaranteed to exist?
            );

            let decode_queue_fns = KhrVideoDecodeQueueFn::load(
                |x| {
                    native_entry
                        .get_instance_proc_addr(native_instance.handle(), x.as_ptr() as *const _)
                        .expect("Must have function pointer") as *const _
                }, // TODO: Is this guaranteed to exist?
            );

            let create_video_session = queue_fns.create_video_session_khr;
            let bind_video_session_memory = queue_fns.bind_video_session_memory_khr;
            let memory_requirements = queue_fns.get_video_session_memory_requirements_khr;

            let mut native_session = VideoSessionKHR::default();
            let mut video_session_requirements = [VideoSessionMemoryRequirementsKHR::default(); 10];
            let mut video_session_count = video_session_requirements.len() as u32;
            let mut allocations = Vec::new();
            let mut bindings = Vec::new();

            create_video_session(native_device.handle(), &video_session_create_info, null(), &mut native_session).result()?;

            unsafe {
                memory_requirements(
                    native_device.handle(),
                    native_session,
                    &mut video_session_count,
                    video_session_requirements.as_mut_ptr(),
                )
                .result()?;

                let video_session_requirements = &video_session_requirements[0..video_session_count as usize];

                for (i, r) in video_session_requirements.iter().enumerate() {
                    let supported_types = r.memory_requirements.memory_type_bits;
                    let best_type = supported_types.trailing_zeros(); // TODO: Better logic to select memory type?

                    let allocation = Allocation::new(device, r.memory_requirements.size, best_type)?;
                    let bind = BindVideoSessionMemoryInfoKHR::default()
                        .memory(allocation.native())
                        .memory_bind_index(i as u32)
                        .memory_size(r.memory_requirements.size)
                        .memory_offset(0);

                    allocations.push(allocation);
                    bindings.push(bind);
                }
            }

            bind_video_session_memory(native_device.handle(), native_session, bindings.len() as u32, bindings.as_ptr()).result()?;

            Ok(Self {
                shared_device,
                native_queue_fns: queue_fns,
                native_decode_queue_fns: decode_queue_fns,
                native_session,
                allocations,
            })
        }
    }

    pub(crate) fn native(&self) -> VideoSessionKHR {
        self.native_session
    }

    pub(crate) fn queue_fns(&self) -> KhrVideoQueueFn {
        self.native_queue_fns.clone()
    }

    pub(crate) fn decode_fns(&self) -> KhrVideoDecodeQueueFn {
        self.native_decode_queue_fns.clone()
    }

    pub(crate) fn device(&self) -> Arc<DeviceShared> {
        self.shared_device.clone()
    }
}

impl Drop for VideoSessionShared {
    fn drop(&mut self) {
        let native_device = self.shared_device.native();
        let destroy_video_session_khr = self.native_queue_fns.destroy_video_session_khr;

        unsafe {
            destroy_video_session_khr(native_device.handle(), self.native_session, null());
        }
    }
}

/// Vulkan-internal state needed for video ops.
pub struct VideoSession {
    shared: Arc<VideoSessionShared>,
}

impl VideoSession {
    pub fn new(device: &Device, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let shared = VideoSessionShared::new(device, stream_inspector)?;

        Ok(Self { shared: Arc::new(shared) })
    }

    pub(crate) fn shared(&self) -> Arc<VideoSessionShared> {
        self.shared.clone()
    }
}

#[cfg(test)]
mod test {
    use crate::device::Device;
    use crate::error::Error;
    use crate::instance::{Instance, InstanceInfo};
    use crate::physicaldevice::PhysicalDevice;
    use crate::video::h264::H264StreamInspector;
    use crate::video::session::VideoSession;

    #[test]
    #[cfg(not(miri))]
    fn create_session() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let h264inspector = H264StreamInspector::new();

        _ = VideoSession::new(&device, &h264inspector)?;

        Ok(())
    }
}
