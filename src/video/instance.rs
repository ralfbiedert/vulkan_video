use crate::device::DeviceShared;
use crate::physicaldevice::PhysicalDeviceShared;
use crate::video::h264::H264StreamInspector;
use crate::video::VideoSessionShared;
use crate::{Device, Error, PhysicalDevice};
use ash::khr::{
    video_decode_queue::DeviceFn as KhrVideoDecodeQueueDeviceFn,
    video_queue::{DeviceFn as KhrVideoQueueDeviceFn, InstanceFn as KhrVideoQueueInstanceFn},
};
use ash::vk::native::StdVideoH264ProfileIdc_STD_VIDEO_H264_PROFILE_IDC_BASELINE;
use ash::vk::{
    ImageUsageFlags, PhysicalDeviceVideoFormatInfoKHR, VideoCapabilitiesKHR, VideoChromaSubsamplingFlagsKHR, VideoCodecOperationFlagsKHR,
    VideoComponentBitDepthFlagsKHR, VideoDecodeCapabilitiesKHR, VideoDecodeH264CapabilitiesKHR, VideoDecodeH264ProfileInfoKHR,
    VideoFormatPropertiesKHR, VideoProfileInfoKHR, VideoProfileListInfoKHR,
};
use std::ptr::null_mut;
use std::sync::Arc;

pub struct VideoInstanceShared {
    shared_physical_device: Arc<PhysicalDeviceShared>,
    shared_device: Arc<DeviceShared>,
    video_instance_fn: KhrVideoQueueInstanceFn,
}

impl VideoInstanceShared {
    pub fn new(device_shared: Arc<DeviceShared>) -> Result<Self, Error> {
        let shared_instance = device_shared.instance();
        let shared_physical_device = device_shared.physical_device();
        let native_instance = shared_instance.native();
        let native_entry = shared_instance.native_entry();

        let video_instance_fn = KhrVideoQueueInstanceFn::load(|x| unsafe {
            native_entry
                .get_instance_proc_addr(native_instance.handle(), x.as_ptr().cast())
                .expect("Must have function pointer") as *const _
        });

        Ok(Self {
            shared_physical_device,
            shared_device: device_shared,
            video_instance_fn,
        })
    }

    pub(crate) fn video_format_properties(&self) -> Result<VideoFormatProperties, Error> {
        let mut video_decode_h264_profile =
            VideoDecodeH264ProfileInfoKHR::default().std_profile_idc(StdVideoH264ProfileIdc_STD_VIDEO_H264_PROFILE_IDC_BASELINE);

        let video_profile = VideoProfileInfoKHR::default()
            .push_next(&mut video_decode_h264_profile)
            .video_codec_operation(VideoCodecOperationFlagsKHR::DECODE_H264)
            .chroma_subsampling(VideoChromaSubsamplingFlagsKHR::TYPE_420)
            .chroma_bit_depth(VideoComponentBitDepthFlagsKHR::TYPE_8)
            .luma_bit_depth(VideoComponentBitDepthFlagsKHR::TYPE_8);

        let get_physical_device_video_format_properties_khr = self.video_instance_fn.get_physical_device_video_format_properties_khr;
        let array = &[video_profile];
        let mut video_profile_list_info = VideoProfileListInfoKHR::default().profiles(array);

        let video_format_info = PhysicalDeviceVideoFormatInfoKHR::default()
            .image_usage(ImageUsageFlags::VIDEO_DECODE_DPB_KHR)
            .push_next(&mut video_profile_list_info);

        let mut num_video_format_properties = 0;

        unsafe {
            (get_physical_device_video_format_properties_khr)(
                self.shared_physical_device.native(),
                &video_format_info,
                &mut num_video_format_properties,
                null_mut(),
            )
            .result()?;

            let mut video_format_properties = VideoFormatProperties::new(num_video_format_properties as usize);

            (get_physical_device_video_format_properties_khr)(
                self.shared_physical_device.native(),
                &video_format_info,
                &mut num_video_format_properties,
                video_format_properties.properties.as_mut_ptr(),
            )
            .result()?;

            Ok(video_format_properties)
        }
    }

    pub(crate) fn video_capabilities(&self) -> Result<VideoCapabilities, Error> {
        let mut video_decode_h264_profile =
            VideoDecodeH264ProfileInfoKHR::default().std_profile_idc(StdVideoH264ProfileIdc_STD_VIDEO_H264_PROFILE_IDC_BASELINE);

        let video_profile = VideoProfileInfoKHR::default()
            .push_next(&mut video_decode_h264_profile)
            .video_codec_operation(VideoCodecOperationFlagsKHR::DECODE_H264)
            .chroma_subsampling(VideoChromaSubsamplingFlagsKHR::TYPE_420)
            .chroma_bit_depth(VideoComponentBitDepthFlagsKHR::TYPE_8)
            .luma_bit_depth(VideoComponentBitDepthFlagsKHR::TYPE_8);

        let mut video_capabilities = VideoCapabilities::new();

        let get_physical_device_video_capabilities = self.video_instance_fn.get_physical_device_video_capabilities_khr;

        unsafe {
            (get_physical_device_video_capabilities)(
                self.shared_device.physical_device().native(),
                &video_profile,
                video_capabilities.caps.as_mut(),
            )
            .result()?;
        }

        Ok(video_capabilities)
    }

    pub(crate) fn shared_device(&self) -> Arc<DeviceShared> {
        Arc::clone(&self.shared_device)
    }
}

pub struct VideoInstance {
    shared: Arc<VideoInstanceShared>,
}

impl VideoInstance {
    pub fn new(device: &Device) -> Result<Self, Error> {
        let shared = VideoInstanceShared::new(device.shared())?;

        Ok(Self { shared: Arc::new(shared) })
    }

    pub fn video_format_properties(&self) -> Result<VideoFormatProperties, Error> {
        self.shared.video_format_properties()
    }

    pub fn video_capabilities(&self) -> Result<VideoCapabilities, Error> {
        self.shared.video_capabilities()
    }

    pub(crate) fn shared(&self) -> Arc<VideoInstanceShared> {
        self.shared.clone()
    }
}

pub struct VideoFormatProperties {
    properties: Vec<VideoFormatPropertiesKHR<'static>>,
}

impl VideoFormatProperties {
    pub(crate) fn new(n: usize) -> Self {
        let properties = vec![VideoFormatPropertiesKHR::default(); n];
        Self { properties }
    }

    pub fn properties(&self) -> &[VideoFormatPropertiesKHR<'static>] {
        &self.properties
    }
}

pub struct VideoCapabilities {
    caps: Box<VideoCapabilitiesKHR<'static>>,
    decode_caps: Box<VideoDecodeCapabilitiesKHR<'static>>,
    decode_caps_h264: Box<VideoDecodeH264CapabilitiesKHR<'static>>,
}

impl VideoCapabilities {
    pub(crate) fn new() -> Self {
        let mut decode_caps = Box::new(VideoDecodeCapabilitiesKHR::default());
        let mut decode_caps_h264 = Box::new(VideoDecodeH264CapabilitiesKHR::default());

        let caps = VideoCapabilitiesKHR::default()
            .push_next(decode_caps.as_mut())
            .push_next(decode_caps_h264.as_mut());

        Self {
            caps: Box::new(caps),
            decode_caps,
            decode_caps_h264,
        }
    }

    pub fn caps(&self) -> &VideoCapabilitiesKHR<'static> {
        &self.caps
    }

    pub fn decode_caps(&self) -> &VideoDecodeCapabilitiesKHR<'static> {
        &self.decode_caps
    }

    pub fn decode_caps_h264(&self) -> &VideoDecodeH264CapabilitiesKHR<'static> {
        &self.decode_caps_h264
    }
}
