use crate::allocation::{Allocation, MemoryTypeIndex};
use crate::device::{Device, DeviceShared};
use crate::error;
use crate::error::{Error, Variant};
use crate::video::h264::H264StreamInspector;
use ash::khr::{
    video_decode_queue::Device as KhrVideoDecodeQueueDevice,
    video_queue::{Device as KhrVideoQueueDevice, Instance as KhrVideoQueueInstance},
};
use ash::vk::native::{StdVideoH264ProfileIdc, StdVideoH264ProfileIdc_STD_VIDEO_H264_PROFILE_IDC_BASELINE};
use ash::vk::{
    self, BindVideoSessionMemoryInfoKHR, ExtensionProperties, Extent2D, Format, ImageUsageFlags, PhysicalDeviceVideoFormatInfoKHR,
    TaggedStructure, VideoCapabilitiesKHR, VideoChromaSubsamplingFlagsKHR, VideoCodecOperationFlagsKHR, VideoComponentBitDepthFlagsKHR,
    VideoDecodeCapabilitiesKHR, VideoDecodeCapabilityFlagsKHR, VideoDecodeH264CapabilitiesKHR, VideoDecodeH264PictureLayoutFlagsKHR,
    VideoDecodeH264ProfileInfoKHR, VideoFormatPropertiesKHR, VideoProfileInfoKHR, VideoProfileListInfoKHR, VideoSessionCreateFlagsKHR,
    VideoSessionCreateInfoKHR, VideoSessionKHR, VideoSessionMemoryRequirementsKHR,
};
use std::ptr::{null, null_mut};
use std::sync::Arc;

pub(crate) struct VideoDecodeCapabilities {
    flags: VideoDecodeCapabilityFlagsKHR,
}
impl From<VideoDecodeCapabilitiesKHR<'_>> for VideoDecodeCapabilities {
    fn from(value: VideoDecodeCapabilitiesKHR) -> Self {
        Self { flags: value.flags }
    }
}
impl VideoDecodeCapabilities {
    pub(crate) fn flags(&self) -> VideoDecodeCapabilityFlagsKHR {
        self.flags
    }
}

pub(crate) struct VideoSessionShared {
    shared_device: Arc<DeviceShared>,
    native_queue_device: KhrVideoQueueDevice,
    native_decode_queue_device: KhrVideoDecodeQueueDevice,
    // native_video_instance_fns: KhrVideoQueueInstanceFn,
    native_session: VideoSessionKHR,
    // allocations: Vec<Allocation>,
    decode_capabilities: VideoDecodeCapabilities,
}

impl VideoSessionShared {
    pub fn new(device: &Device, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let shared_device = device.shared();
        let shared_instance = shared_device.instance();

        let native_device = shared_device.native();
        let native_instance = shared_instance.native();
        let native_entry = shared_instance.native_entry();

        let extension_name = c"VK_STD_vulkan_video_codec_h264_decode";
        let extension_version = vk::make_api_version(0, 1, 0, 0);

        let extensions_names = ExtensionProperties::default()
            .spec_version(extension_version)
            .extension_name(extension_name)?;

        let profiles = stream_inspector.profiles();

        let queue_family_index = shared_device
            .physical_device()
            .queue_family_infos()
            .any_decode()
            .ok_or_else(|| error!(Variant::QueueNotFound))?;

        let video_session_create_info = VideoSessionCreateInfoKHR::default()
            .queue_family_index(queue_family_index)
            .flags(VideoSessionCreateFlagsKHR::empty())
            .video_profile(&profiles.info)
            .picture_format(Format::G8_B8R8_2PLANE_420_UNORM)
            .max_coded_extent(Extent2D { width: 512, height: 512 })
            .reference_picture_format(Format::G8_B8R8_2PLANE_420_UNORM)
            .max_dpb_slots(17)
            .max_active_reference_pictures(16)
            .std_header_version(&extensions_names);

        let result = unsafe {
            let queue_device = KhrVideoQueueDevice::new(&native_instance, &native_device);

            let decode_queue_device = KhrVideoDecodeQueueDevice::new(&native_instance, &native_device);

            let video_instance = KhrVideoQueueInstance::new(&native_entry, &native_instance);

            let mut video_decode_h264_profile =
                VideoDecodeH264ProfileInfoKHR::default().std_profile_idc(StdVideoH264ProfileIdc_STD_VIDEO_H264_PROFILE_IDC_BASELINE);

            let video_profile = VideoProfileInfoKHR::default()
                .extend(&mut video_decode_h264_profile)
                .video_codec_operation(VideoCodecOperationFlagsKHR::DECODE_H264)
                .chroma_subsampling(VideoChromaSubsamplingFlagsKHR::TYPE_420)
                .chroma_bit_depth(VideoComponentBitDepthFlagsKHR::TYPE_8)
                .luma_bit_depth(VideoComponentBitDepthFlagsKHR::TYPE_8);

            let mut video_decode_h264_capabilities = VideoDecodeH264CapabilitiesKHR::default();

            let mut video_decode_capabilities = VideoDecodeCapabilitiesKHR::default();

            // Does this order matter?  It seems to work without relevant validation failures either way.
            let mut video_capabilities = VideoCapabilitiesKHR::default()
                .extend(&mut video_decode_capabilities)
                .extend(&mut video_decode_h264_capabilities);

            video_instance.get_physical_device_video_capabilities(
                shared_device.physical_device().native(),
                &video_profile,
                &mut video_capabilities,
            )?;

            let array = &[video_profile];

            let mut video_profile_list_info = VideoProfileListInfoKHR::default().profiles(array);

            let video_format_info = PhysicalDeviceVideoFormatInfoKHR::default()
                .image_usage(ImageUsageFlags::VIDEO_DECODE_DPB_KHR)
                .extend(&mut video_profile_list_info);

            let num_video_format_properties = video_instance
                .get_physical_device_video_format_properties_len(shared_device.physical_device().native(), &video_format_info)?;

            let mut video_format_properties = vec![VideoFormatPropertiesKHR::default(); num_video_format_properties];

            video_instance.get_physical_device_video_format_properties(
                shared_device.physical_device().native(),
                &video_format_info,
                &mut video_format_properties,
            )?;

            let mut allocations = Vec::new();
            let mut bindings = Vec::new();

            let native_session = queue_device.create_video_session(&video_session_create_info, None)?;

            let video_session_count = queue_device.get_video_session_memory_requirements_len(native_session)?;

            let mut video_session_requirements = vec![VideoSessionMemoryRequirementsKHR::default(); video_session_count];

            queue_device.get_video_session_memory_requirements(native_session, &mut video_session_requirements)?;

            let video_session_requirements = &video_session_requirements[0..video_session_count as usize];

            for (i, r) in video_session_requirements.iter().enumerate() {
                let supported_types = r.memory_requirements.memory_type_bits;
                let best_type = MemoryTypeIndex::new(supported_types.trailing_zeros()); // TODO: Better logic to select memory type?

                let allocation = Allocation::new(device, r.memory_requirements.size, best_type)?;
                let bind = BindVideoSessionMemoryInfoKHR::default()
                    .memory(allocation.native())
                    .memory_bind_index(i as u32)
                    .memory_size(r.memory_requirements.size)
                    .memory_offset(0);

                allocations.push(allocation);
                bindings.push(bind);
            }

            queue_device.bind_video_session_memory(native_session, &bindings)?;

            Ok(Self {
                shared_device,
                native_queue_device: queue_device,
                native_decode_queue_device: decode_queue_device,
                // native_video_instance_fns: video_instance_fn,
                native_session,
                // allocations,
                decode_capabilities: video_decode_capabilities.into(),
            })
        };
        result
    }

    pub(crate) fn native(&self) -> VideoSessionKHR {
        self.native_session
    }

    pub(crate) fn queue_device(&self) -> KhrVideoQueueDevice {
        self.native_queue_device.clone()
    }

    pub(crate) fn decode_queue_device(&self) -> KhrVideoDecodeQueueDevice {
        self.native_decode_queue_device.clone()
    }

    // pub(crate) fn video_instance(&self) -> KhrVideoQueueInstance {
    //     self.native_video_instance.clone()
    // }

    pub(crate) fn device(&self) -> Arc<DeviceShared> {
        self.shared_device.clone()
    }

    pub(crate) fn decode_capabilities(&self) -> &VideoDecodeCapabilities {
        &self.decode_capabilities
    }
}

impl Drop for VideoSessionShared {
    fn drop(&mut self) {
        unsafe {
            self.native_queue_device.destroy_video_session(self.native_session, None);
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
