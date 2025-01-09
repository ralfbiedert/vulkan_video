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
    pub fn new(shared_session: Arc<VideoSessionShared>, _stream_inspector: &H264StreamInspector) -> Result<Self, Error> {
        let native_session = shared_session.native();
        let native_device = shared_session.device().native();
        let native_queue_fns = shared_session.queue_fns();

        let hrd = StdVideoH264HrdParameters {
            cpb_cnt_minus1: 0,
            bit_rate_scale: 0,
            cpb_size_scale: 0,
            reserved1: 0,
            bit_rate_value_minus1: Default::default(),
            cpb_size_value_minus1: Default::default(),
            cbr_flag: Default::default(),
            initial_cpb_removal_delay_length_minus1: 23,
            cpb_removal_delay_length_minus1: 0,
            dpb_output_delay_length_minus1: 0,
            time_offset_length: 0,
        };

        let mut vui_flags = StdVideoH264SpsVuiFlags {
            _bitfield_align_1: [],
            _bitfield_1: Default::default(),
            __bindgen_padding_0: 0,
        };

        vui_flags.set_video_signal_type_present_flag(1);
        vui_flags.set_color_description_present_flag(1);
        vui_flags.set_bitstream_restriction_flag(1);

        let vui = StdVideoH264SequenceParameterSetVui {
            flags: vui_flags,
            aspect_ratio_idc: 0,
            sar_width: 1,
            sar_height: 1,
            video_format: 2,
            colour_primaries: 5,
            transfer_characteristics: 6,
            matrix_coefficients: 6,
            num_units_in_tick: 0,
            time_scale: 0,
            max_num_reorder_frames: 0,
            max_dec_frame_buffering: 16,
            chroma_sample_loc_type_top_field: 0,
            chroma_sample_loc_type_bottom_field: 0,
            reserved1: 0,
            pHrdParameters: &hrd,
        };

        let mut flags = StdVideoH264SpsFlags {
            _bitfield_align_1: [],
            _bitfield_1: Default::default(),
            __bindgen_padding_0: 0,
        };

        flags.set_frame_mbs_only_flag(1);
        flags.set_vui_parameters_present_flag(1);
        flags.set_direct_8x8_inference_flag(1);

        let sps_info = StdVideoH264SequenceParameterSet {
            flags,
            profile_idc: 100,
            level_idc: 8,
            chroma_format_idc: 1,
            seq_parameter_set_id: 0,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            log2_max_frame_num_minus4: 0,
            pic_order_cnt_type: 2,
            offset_for_non_ref_pic: 0,
            offset_for_top_to_bottom_field: 0,
            log2_max_pic_order_cnt_lsb_minus4: 0,
            num_ref_frames_in_pic_order_cnt_cycle: 0,
            max_num_ref_frames: 0,
            reserved1: 0,
            pic_width_in_mbs_minus1: 31,
            pic_height_in_map_units_minus1: 31,
            frame_crop_left_offset: 0,
            frame_crop_right_offset: 0,
            frame_crop_top_offset: 0,
            frame_crop_bottom_offset: 0,
            reserved2: 0,
            pOffsetForRefFrame: null(),
            pScalingLists: null(),
            pSequenceParameterSetVui: &vui,
        };

        let mut pps_flags = StdVideoH264PpsFlags {
            _bitfield_align_1: Default::default(),
            _bitfield_1: Default::default(),
            __bindgen_padding_0: Default::default(),
        };

        pps_flags.set_transform_8x8_mode_flag(1);
        pps_flags.set_deblocking_filter_control_present_flag(1);
        pps_flags.set_entropy_coding_mode_flag(1);

        let pps_info = StdVideoH264PictureParameterSet {
            flags: pps_flags,
            seq_parameter_set_id: 0,
            pic_parameter_set_id: 0,
            num_ref_idx_l0_default_active_minus1: 0,
            num_ref_idx_l1_default_active_minus1: 0,
            weighted_bipred_idc: 0,
            pic_init_qp_minus26: -6,
            pic_init_qs_minus26: 0,
            chroma_qp_index_offset: 0,
            second_chroma_qp_index_offset: 0,
            pScalingLists: null(),
        };

        let sps_array = &[sps_info];
        let pps_array = &[pps_info];

        let create_info = VideoDecodeH264SessionParametersAddInfoKHR::default()
            .std_sp_ss(sps_array)
            .std_pp_ss(pps_array);

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
