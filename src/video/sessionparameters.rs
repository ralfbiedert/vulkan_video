use crate::error::Error;
use crate::video::h264::H264StreamInspector;
use crate::video::session::{VideoSession, VideoSessionShared};
use ash::vk::native::{
    StdVideoH264HrdParameters, StdVideoH264PictureParameterSet, StdVideoH264PpsFlags, StdVideoH264ScalingLists,
    StdVideoH264SequenceParameterSet, StdVideoH264SequenceParameterSetVui, StdVideoH264SpsFlags, StdVideoH264SpsVuiFlags,
};
use ash::vk::{
    VideoDecodeH264SessionParametersAddInfoKHR, VideoDecodeH264SessionParametersCreateInfoKHR, VideoSessionParametersCreateInfoKHR,
    VideoSessionParametersKHR, VideoSessionParametersUpdateInfoKHR,
};
use std::ptr::{addr_of, addr_of_mut, null};
use std::sync::Arc;

pub(crate) struct VideoSessionParametersShared {
    shared_session: Arc<VideoSessionShared>,
    native_parameters: VideoSessionParametersKHR,
}

impl VideoSessionParametersShared {
    pub fn new(shared_session: Arc<VideoSessionShared>, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let native_session = shared_session.native();
        let native_device = shared_session.device().native();
        let native_queue_fns = shared_session.queue_fns();

        // bind nested pointers with lifetime safety to stack ownership
        let sps1 = stream_inspector.h264_sps();
        let sps2 = sps1.step2();
        let sps3 = sps2.step3();

        let pps1 = stream_inspector.h264_pps();
        let pps2 = pps1.step2();

        let create_info = VideoDecodeH264SessionParametersAddInfoKHR::default()
            .std_sp_ss(sps3.array())
            .std_pp_ss(pps2.array());

        let mut video_decode_h264session_parameters_create_info = VideoDecodeH264SessionParametersCreateInfoKHR::default()
            .max_std_sps_count(32)
            .max_std_pps_count(256)
            .parameters_add_info(&create_info);

        let session_create_info = VideoSessionParametersCreateInfoKHR::default()
            .video_session(native_session)
            .push_next(&mut video_decode_h264session_parameters_create_info);

        unsafe {
            let mut native_parameters = VideoSessionParametersKHR::null();
            let create_video_session_parameters = native_queue_fns.create_video_session_parameters_khr;
            // let update_video_session_parameters = native_queue_fns.update_video_session_parameters_khr;

            create_video_session_parameters(native_device.handle(), &session_create_info, null(), &mut native_parameters).result()?;
            // update_video_session_parameters(native_device.handle(), native_parameters, &update).result()?;

            Ok(Self {
                shared_session,
                native_parameters,
            })
        }
    }

    pub(crate) fn native(&self) -> VideoSessionParametersKHR {
        self.native_parameters
    }

    pub(crate) fn video_session(&self) -> Arc<VideoSessionShared> {
        self.shared_session.clone()
    }
}

impl Drop for VideoSessionParametersShared {
    fn drop(&mut self) {
        let queue_fns = self.shared_session.queue_fns();
        let native_device = self.shared_session.device().native();

        let destroy_video_session_parameters_khr = queue_fns.destroy_video_session_parameters_khr;

        unsafe {
            destroy_video_session_parameters_khr(native_device.handle(), self.native_parameters, null());
        }
    }
}

/// Vulkan-internal state needed for operating on a single video frame.
pub struct VideoSessionParameters {
    shared: Arc<VideoSessionParametersShared>,
}

impl VideoSessionParameters {
    pub fn new(session: &VideoSession, stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let shared = VideoSessionParametersShared::new(session.shared(), stream_inspector)?;

        Ok(Self { shared: Arc::new(shared) })
    }

    pub(crate) fn shared(&self) -> Arc<VideoSessionParametersShared> {
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
    use crate::video::sessionparameters::VideoSessionParameters;

    #[test]
    #[cfg(not(miri))]
    fn create_session_parameters() -> Result<(), Error> {
        let instance_info = InstanceInfo::new().app_name("MyApp")?.app_version(100).validation(true);
        let instance = Instance::new(&instance_info)?;
        let physical_device = PhysicalDevice::new_any(&instance)?;
        let device = Device::new(&physical_device)?;
        let h264inspector = H264StreamInspector::new();
        let session = VideoSession::new(&device, &h264inspector)?;

        _ = VideoSessionParameters::new(&session, &h264inspector)?;

        Ok(())
    }
}
