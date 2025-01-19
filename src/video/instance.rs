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
    ImageUsageFlags, PhysicalDeviceVideoFormatInfoKHR, VideoChromaSubsamplingFlagsKHR, VideoCodecOperationFlagsKHR,
    VideoComponentBitDepthFlagsKHR, VideoDecodeH264ProfileInfoKHR, VideoFormatPropertiesKHR, VideoProfileInfoKHR, VideoProfileListInfoKHR,
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

    pub(crate) fn xxx(&self) -> Result<(), Error> {
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

            let mut video_format_properties = vec![VideoFormatPropertiesKHR::default(); num_video_format_properties as usize];

            (get_physical_device_video_format_properties_khr)(
                self.shared_physical_device.native(),
                &video_format_info,
                &mut num_video_format_properties,
                video_format_properties.as_mut_ptr(),
            )
            .result()?;
        }

        Ok(())
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

    pub fn xxx(&self) -> Result<(), Error> {
        self.shared.xxx()
    }

    pub(crate) fn shared(&self) -> Arc<VideoInstanceShared> {
        self.shared.clone()
    }
}
