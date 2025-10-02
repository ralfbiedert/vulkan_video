use crate::error::Error;
use crate::video::h264::H264StreamInspector;
use crate::video::session::{VideoSession, VideoSessionShared};

use ash::vk::{VideoSessionParametersCreateInfoKHR, VideoSessionParametersKHR};
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

        let mut native_parameters = VideoSessionParametersKHR::null();

        stream_inspector.run_with_create_info(|video_decode_h264session_parameters_create_info| {
            let session_create_info = VideoSessionParametersCreateInfoKHR::default()
                .video_session(native_session)
                .push_next(video_decode_h264session_parameters_create_info);

            let create_video_session_parameters = native_queue_fns.create_video_session_parameters_khr;
            unsafe {
                create_video_session_parameters(native_device.handle(), &session_create_info, null(), &mut native_parameters).result()
            }
        })?;

        Ok(Self {
            shared_session,
            native_parameters,
        })
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
