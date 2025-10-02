use core::ptr::null;

use ash::vk::native::{
    StdVideoH264HrdParameters, StdVideoH264PictureParameterSet, StdVideoH264PpsFlags, StdVideoH264ScalingLists,
    StdVideoH264SequenceParameterSet, StdVideoH264SequenceParameterSetVui, StdVideoH264SpsFlags, StdVideoH264SpsVuiFlags,
};
use ash::vk::{VideoDecodeH264SessionParametersAddInfoKHR, VideoDecodeH264SessionParametersCreateInfoKHR};
use h264_reader::nal::{pps::PicParameterSet, sps::SeqParameterSet};

use crate::video::h264::H264StreamInspector;

impl H264StreamInspector {
    pub fn run_with_create_info<T>(&self, mut f: impl FnMut(&mut VideoDecodeH264SessionParametersCreateInfoKHR) -> T) -> T {
        // sps structs are nested 3-deep
        let sps1: Vec<_> = self.context().sps().map(SpsInfo1::new).collect();
        let sps2: Vec<_> = sps1.iter().map(SpsInfo1::step2).collect();
        let sps3: Vec<_> = sps2.iter().map(SpsInfo2::step3).collect();

        // pps structs are nested 2-deep
        let pps1: Vec<_> = self.context().pps().map(PpsInfo1::new).collect();
        let pps2: Vec<_> = pps1.iter().map(PpsInfo1::step2).collect();

        let create_info = VideoDecodeH264SessionParametersAddInfoKHR::default()
            .std_sp_ss(&sps3)
            .std_pp_ss(&pps2);

        let mut video_decode_h264session_parameters_create_info = VideoDecodeH264SessionParametersCreateInfoKHR::default()
            .max_std_sps_count(32)
            .max_std_pps_count(256)
            .parameters_add_info(&create_info);

        f(&mut video_decode_h264session_parameters_create_info)
    }
}

// Builders for Vulkan parameters containing nested pointers
// Adds lifetime safety

struct SpsInfo1<'a> {
    sps: &'a SeqParameterSet,
    p_hrd_parameters: Option<StdVideoH264HrdParameters>,
}
impl<'a> SpsInfo1<'a> {
    fn new(sps: &'a SeqParameterSet) -> Self {
        let p_hrd_parameters = Some(StdVideoH264HrdParameters {
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
        });
        SpsInfo1 { sps, p_hrd_parameters }
    }
    fn step2<'b>(&'b self) -> SpsInfo2<'b> {
        let p_scaling_lists = None;
        let p_sequence_parameter_set_vui = self.sps.vui_parameters.as_ref().map(|vui| {
            let mut flags = StdVideoH264SpsVuiFlags {
                _bitfield_align_1: [],
                _bitfield_1: Default::default(),
                __bindgen_padding_0: 0,
            };

            flags.set_video_signal_type_present_flag(1);
            flags.set_color_description_present_flag(1);
            flags.set_bitstream_restriction_flag(1);

            StdVideoH264SequenceParameterSetVui {
                flags,
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
                pHrdParameters: self.p_hrd_parameters.as_ref().map_or(null(), |p| p),
            }
        });

        SpsInfo2 {
            sps: self.sps,
            p_scaling_lists,
            p_sequence_parameter_set_vui,
        }
    }
}

struct SpsInfo2<'a> {
    sps: &'a SeqParameterSet,
    p_scaling_lists: Option<StdVideoH264ScalingLists>,
    p_sequence_parameter_set_vui: Option<StdVideoH264SequenceParameterSetVui>,
}

impl SpsInfo2<'_> {
    fn step3(&self) -> StdVideoH264SequenceParameterSet {
        let mut flags = StdVideoH264SpsFlags {
            _bitfield_align_1: [],
            _bitfield_1: Default::default(),
            __bindgen_padding_0: 0,
        };
        flags.set_frame_mbs_only_flag(1);
        flags.set_vui_parameters_present_flag(1);
        flags.set_direct_8x8_inference_flag(1);

        StdVideoH264SequenceParameterSet {
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
            pScalingLists: self.p_scaling_lists.as_ref().map_or(null(), |p| p),
            pSequenceParameterSetVui: self.p_sequence_parameter_set_vui.as_ref().map_or(null(), |p| p),
        }
    }
}

struct PpsInfo1<'a> {
    pps: &'a PicParameterSet,
    p_scaling_lists: Option<StdVideoH264ScalingLists>,
}
impl<'a> PpsInfo1<'a> {
    fn new(pps: &'a PicParameterSet) -> Self {
        let p_scaling_lists = None;
        PpsInfo1 { pps, p_scaling_lists }
    }
    fn step2(&self) -> StdVideoH264PictureParameterSet {
        let mut pps_flags = StdVideoH264PpsFlags {
            _bitfield_align_1: Default::default(),
            _bitfield_1: Default::default(),
            __bindgen_padding_0: Default::default(),
        };

        pps_flags.set_transform_8x8_mode_flag(1);
        pps_flags.set_deblocking_filter_control_present_flag(1);
        pps_flags.set_entropy_coding_mode_flag(1);

        StdVideoH264PictureParameterSet {
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
            pScalingLists: self.p_scaling_lists.as_ref().map_or(null(), |p| p),
        }
    }
}
