use core::ptr::null;

use ash::vk::native::{
    StdVideoH264HrdParameters, StdVideoH264PictureParameterSet, StdVideoH264PpsFlags, StdVideoH264ScalingLists,
    StdVideoH264SequenceParameterSet, StdVideoH264SequenceParameterSetVui, StdVideoH264SpsFlags, StdVideoH264SpsVuiFlags,
};
use ash::vk::{VideoDecodeH264SessionParametersAddInfoKHR, VideoDecodeH264SessionParametersCreateInfoKHR};
use h264_reader::nal::pps::PicScalingMatrix;
use h264_reader::nal::sps::ScalingList;
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
        let p_hrd_parameters = sps
            .vui_parameters
            .as_ref()
            .and_then(|vui| vui.nal_hrd_parameters.as_ref().or(vui.vcl_hrd_parameters.as_ref()))
            .map(|hrd| {
                let mut bit_rate_value_minus1 = [0; 32];
                let mut cpb_size_value_minus1 = [0; 32];
                let mut cbr_flag = [0; 32];
                assert!((1..=32).contains(&hrd.cpb_specs.len()));
                for (i, cpb) in hrd.cpb_specs.iter().enumerate() {
                    bit_rate_value_minus1[i] = cpb.bit_rate_value_minus1;
                    cpb_size_value_minus1[i] = cpb.cpb_size_value_minus1;
                    cbr_flag[i] = cpb.cbr_flag as u8;
                }
                StdVideoH264HrdParameters {
                    cpb_cnt_minus1: hrd.cpb_specs.len() as u8 - 1,
                    bit_rate_scale: hrd.bit_rate_scale,
                    cpb_size_scale: hrd.cpb_size_scale,
                    reserved1: 0,
                    bit_rate_value_minus1,
                    cpb_size_value_minus1,
                    cbr_flag,
                    initial_cpb_removal_delay_length_minus1: hrd.initial_cpb_removal_delay_length_minus1 as u32,
                    cpb_removal_delay_length_minus1: hrd.cpb_removal_delay_length_minus1 as u32,
                    dpb_output_delay_length_minus1: hrd.dpb_output_delay_length_minus1 as u32,
                    time_offset_length: hrd.time_offset_length as u32,
                }
            });
        SpsInfo1 { sps, p_hrd_parameters }
    }
    fn step2<'b>(&'b self) -> SpsInfo2<'b> {
        let p_scaling_lists = self
            .sps
            .chroma_info
            .scaling_matrix
            .as_ref()
            .map(|scaling_matrix| scaling_list(&scaling_matrix.scaling_list4x4, &scaling_matrix.scaling_list8x8));
        let p_sequence_parameter_set_vui = self.sps.vui_parameters.as_ref().map(|vui| {
            let mut flags = StdVideoH264SpsVuiFlags {
                _bitfield_align_1: [],
                _bitfield_1: Default::default(),
                __bindgen_padding_0: 0,
            };
            // flags.set_overscan_info_present_flag(vui.overscan_info_present);
            flags.set_overscan_appropriate_flag(
                (vui.overscan_appropriate == h264_reader::nal::sps::OverscanAppropriate::Appropriate) as u32,
            );
            flags.set_nal_hrd_parameters_present_flag(vui.nal_hrd_parameters.is_some() as u32);
            flags.set_vcl_hrd_parameters_present_flag(vui.vcl_hrd_parameters.is_some() as u32);

            // aspect_ratio_info
            let mut aspect_ratio_idc = 0;
            let mut sar_width = 1;
            let mut sar_height = 1;
            if let Some(aspect_ratio_info) = &vui.aspect_ratio_info {
                if let Some((w, h)) = aspect_ratio_info.get() {
                    sar_width = w;
                    sar_height = h;
                }
                aspect_ratio_idc = aspect_ratio_info.to_u8() as u32;
            }
            flags.set_aspect_ratio_info_present_flag(vui.aspect_ratio_info.is_some() as u32);

            // video_signal_type
            let mut video_format = h264_reader::nal::sps::VideoFormat::NTSC.to_u8();
            let mut colour_primaries = 2;
            let mut transfer_characteristics = 2;
            let mut matrix_coefficients = 2;
            if let Some(video_signal_type) = &vui.video_signal_type {
                video_format = video_signal_type.video_format.to_u8();
                if let Some(colour_description) = &video_signal_type.colour_description {
                    colour_primaries = colour_description.colour_primaries;
                    transfer_characteristics = colour_description.transfer_characteristics;
                    matrix_coefficients = colour_description.matrix_coefficients;
                }
                flags.set_color_description_present_flag(video_signal_type.colour_description.is_some() as u32);
                flags.set_video_full_range_flag(video_signal_type.video_full_range_flag as u32);
            }
            flags.set_video_signal_type_present_flag(vui.video_signal_type.is_some() as u32);

            // timing_info
            let mut num_units_in_tick = 0;
            let mut time_scale = 0;
            if let Some(timing_info) = &vui.timing_info {
                num_units_in_tick = timing_info.num_units_in_tick;
                time_scale = timing_info.time_scale;
                flags.set_fixed_frame_rate_flag(timing_info.fixed_frame_rate_flag as u32);
            }
            flags.set_timing_info_present_flag(vui.timing_info.is_some() as u32);

            // bitstream_restrictions
            let mut max_num_reorder_frames = 0;
            let mut max_dec_frame_buffering = 0;
            if let Some(bitstream_restrictions) = &vui.bitstream_restrictions {
                max_num_reorder_frames = bitstream_restrictions.max_num_reorder_frames as u8;
                max_dec_frame_buffering = bitstream_restrictions.max_dec_frame_buffering as u8;
            }
            flags.set_bitstream_restriction_flag(vui.bitstream_restrictions.is_some() as u32);

            // chroma_loc_info
            let mut chroma_sample_loc_type_top_field = 0;
            let mut chroma_sample_loc_type_bottom_field = 0;
            if let Some(chroma_loc_info) = &vui.chroma_loc_info {
                chroma_sample_loc_type_top_field = chroma_loc_info.chroma_sample_loc_type_top_field as u8;
                chroma_sample_loc_type_bottom_field = chroma_loc_info.chroma_sample_loc_type_bottom_field as u8;
            }
            flags.set_chroma_loc_info_present_flag(vui.chroma_loc_info.is_some() as u32);

            StdVideoH264SequenceParameterSetVui {
                flags,
                aspect_ratio_idc,
                sar_width,
                sar_height,
                video_format,
                colour_primaries,
                transfer_characteristics,
                matrix_coefficients,
                num_units_in_tick,
                time_scale,
                max_num_reorder_frames,
                max_dec_frame_buffering,
                chroma_sample_loc_type_top_field,
                chroma_sample_loc_type_bottom_field,
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
        flags.set_constraint_set0_flag(self.sps.constraint_flags.flag0() as u32);
        flags.set_constraint_set1_flag(self.sps.constraint_flags.flag1() as u32);
        flags.set_constraint_set2_flag(self.sps.constraint_flags.flag2() as u32);
        flags.set_constraint_set3_flag(self.sps.constraint_flags.flag3() as u32);
        flags.set_constraint_set4_flag(self.sps.constraint_flags.flag4() as u32);
        flags.set_constraint_set5_flag(self.sps.constraint_flags.flag5() as u32);

        flags.set_direct_8x8_inference_flag(self.sps.direct_8x8_inference_flag as u32);

        // frame_mbs_flags
        use h264_reader::nal::sps::FrameMbsFlags;
        match &self.sps.frame_mbs_flags {
            FrameMbsFlags::Frames => flags.set_frame_mbs_only_flag(1),
            FrameMbsFlags::Fields {
                mb_adaptive_frame_field_flag,
            } => flags.set_mb_adaptive_frame_field_flag(*mb_adaptive_frame_field_flag as u32),
        }

        flags.set_separate_colour_plane_flag(self.sps.chroma_info.separate_colour_plane_flag as u32);
        flags.set_gaps_in_frame_num_value_allowed_flag(self.sps.gaps_in_frame_num_value_allowed_flag as u32);
        flags.set_qpprime_y_zero_transform_bypass_flag(self.sps.chroma_info.qpprime_y_zero_transform_bypass_flag as u32);

        // frame_cropping
        let mut frame_crop_left_offset = 0;
        let mut frame_crop_right_offset = 0;
        let mut frame_crop_top_offset = 0;
        let mut frame_crop_bottom_offset = 0;
        if let Some(frame_cropping) = &self.sps.frame_cropping {
            frame_crop_left_offset = frame_cropping.left_offset;
            frame_crop_right_offset = frame_cropping.right_offset;
            frame_crop_top_offset = frame_cropping.top_offset;
            frame_crop_bottom_offset = frame_cropping.bottom_offset;
        }
        flags.set_frame_cropping_flag(self.sps.frame_cropping.is_some() as u32);

        flags.set_seq_scaling_matrix_present_flag(self.p_scaling_lists.is_some() as u32);
        flags.set_vui_parameters_present_flag(self.p_sequence_parameter_set_vui.is_some() as u32);

        // pic_order_cnt
        use h264_reader::nal::sps::PicOrderCntType;
        let mut offset_for_non_ref_pic = 0;
        let mut offset_for_top_to_bottom_field = 0;
        let mut log2_max_pic_order_cnt_lsb_minus4 = 0;
        let mut p_offset_for_ref_frame = None;
        let pic_order_cnt_type = match &self.sps.pic_order_cnt {
            PicOrderCntType::TypeZero {
                log2_max_pic_order_cnt_lsb_minus4: _0,
            } => {
                log2_max_pic_order_cnt_lsb_minus4 = *_0;
                0
            }
            PicOrderCntType::TypeOne {
                offsets_for_ref_frame,
                delta_pic_order_always_zero_flag,
                offset_for_non_ref_pic: _0,
                offset_for_top_to_bottom_field: _1,
            } => {
                offset_for_non_ref_pic = *_0;
                offset_for_top_to_bottom_field = *_1;
                flags.set_delta_pic_order_always_zero_flag(*delta_pic_order_always_zero_flag as u32);
                p_offset_for_ref_frame = Some(offsets_for_ref_frame.as_slice());
                1
            }
            PicOrderCntType::TypeTwo => 2,
        };

        let profile_idc: u8 = self.sps.profile_idc.into();
        StdVideoH264SequenceParameterSet {
            flags,
            profile_idc: profile_idc as u32,
            level_idc: self.sps.level_idc as u32,
            chroma_format_idc: self.sps.chroma_info.chroma_format.to_u32(),
            seq_parameter_set_id: self.sps.seq_parameter_set_id.id(),
            bit_depth_luma_minus8: self.sps.chroma_info.bit_depth_luma_minus8,
            bit_depth_chroma_minus8: self.sps.chroma_info.bit_depth_chroma_minus8,
            log2_max_frame_num_minus4: self.sps.log2_max_frame_num_minus4,
            pic_order_cnt_type,
            offset_for_non_ref_pic,
            offset_for_top_to_bottom_field,
            log2_max_pic_order_cnt_lsb_minus4,
            num_ref_frames_in_pic_order_cnt_cycle: p_offset_for_ref_frame.map_or(0, |p| p.len() as u8),
            max_num_ref_frames: self.sps.max_num_ref_frames as u8,
            reserved1: 0,
            pic_width_in_mbs_minus1: self.sps.pic_width_in_mbs_minus1,
            pic_height_in_map_units_minus1: self.sps.pic_height_in_map_units_minus1,
            frame_crop_left_offset,
            frame_crop_right_offset,
            frame_crop_top_offset,
            frame_crop_bottom_offset,
            reserved2: 0,
            pOffsetForRefFrame: p_offset_for_ref_frame.map_or(null(), |p| p.as_ptr()),
            pScalingLists: self.p_scaling_lists.as_ref().map_or(null(), |p| p),
            pSequenceParameterSetVui: self.p_sequence_parameter_set_vui.as_ref().map_or(null(), |p| p),
        }
    }
}

const SCALING_LIST4X4_NUM_ELEMENTS: usize = 16;
const SCALING_LIST8X8_NUM_ELEMENTS: usize = 64;
const SCALING_LIST4X4_NUM_LISTS: usize = 6;
const SCALING_LIST8X8_NUM_LISTS: usize = 6;
fn scaling_list(
    scaling_list4x4: &[ScalingList<SCALING_LIST4X4_NUM_ELEMENTS>],
    scaling_list8x8: &[ScalingList<SCALING_LIST8X8_NUM_ELEMENTS>],
) -> StdVideoH264ScalingLists {
    assert!(scaling_list4x4.len() <= SCALING_LIST4X4_NUM_LISTS);
    assert!(scaling_list8x8.len() <= SCALING_LIST8X8_NUM_LISTS);
    use h264_reader::nal::sps::ScalingList;
    let mut scaling_list_present_mask = 0;
    let mut use_default_scaling_matrix_mask = 0;
    let mut scaling_list_4x4 = [[0; SCALING_LIST4X4_NUM_ELEMENTS]; SCALING_LIST4X4_NUM_LISTS];
    for (i, scaling_list) in scaling_list4x4.iter().enumerate() {
        match scaling_list {
            ScalingList::NotPresent => scaling_list_present_mask |= 1 << i,
            ScalingList::UseDefault => use_default_scaling_matrix_mask |= 1 << i,
            ScalingList::List(scaling_list) => scaling_list_4x4[i] = scaling_list.map(|n| n.get()),
        }
    }
    let mut scaling_list_8x8 = [[0; SCALING_LIST8X8_NUM_ELEMENTS]; SCALING_LIST8X8_NUM_LISTS];
    for (i, scaling_list) in scaling_list8x8.iter().enumerate() {
        match scaling_list {
            ScalingList::NotPresent => scaling_list_present_mask |= 1 << (i + SCALING_LIST4X4_NUM_LISTS),
            ScalingList::UseDefault => use_default_scaling_matrix_mask |= 1 << (i + SCALING_LIST4X4_NUM_LISTS),
            ScalingList::List(scaling_list) => scaling_list_8x8[i] = scaling_list.map(|n| n.get()),
        }
    }
    StdVideoH264ScalingLists {
        scaling_list_present_mask,
        use_default_scaling_matrix_mask,
        ScalingList4x4: scaling_list_4x4,
        ScalingList8x8: scaling_list_8x8,
    }
}

struct PpsInfo1<'a> {
    pps: &'a PicParameterSet,
    p_scaling_lists: Option<StdVideoH264ScalingLists>,
}
impl<'a> PpsInfo1<'a> {
    fn new(pps: &'a PicParameterSet) -> Self {
        let p_scaling_lists = pps
            .extension
            .as_ref()
            .and_then(|ex| ex.pic_scaling_matrix.as_ref())
            .map(|scaling_matrix| {
                scaling_list(
                    &scaling_matrix.scaling_list4x4,
                    scaling_matrix.scaling_list8x8.as_ref().map_or(&[], |scaling_list| scaling_list),
                )
            });
        PpsInfo1 { pps, p_scaling_lists }
    }
    fn step2(&self) -> StdVideoH264PictureParameterSet {
        let mut pps_flags = StdVideoH264PpsFlags {
            _bitfield_align_1: Default::default(),
            _bitfield_1: Default::default(),
            __bindgen_padding_0: Default::default(),
        };
        if let Some(ex) = &self.pps.extension {
            pps_flags.set_transform_8x8_mode_flag(ex.transform_8x8_mode_flag as u32);
        }
        pps_flags.set_redundant_pic_cnt_present_flag(self.pps.redundant_pic_cnt_present_flag as u32);
        pps_flags.set_constrained_intra_pred_flag(self.pps.constrained_intra_pred_flag as u32);
        pps_flags.set_deblocking_filter_control_present_flag(self.pps.deblocking_filter_control_present_flag as u32);
        pps_flags.set_weighted_pred_flag(self.pps.weighted_pred_flag as u32);
        pps_flags.set_bottom_field_pic_order_in_frame_present_flag(self.pps.bottom_field_pic_order_in_frame_present_flag as u32);
        pps_flags.set_entropy_coding_mode_flag(self.pps.entropy_coding_mode_flag as u32);
        pps_flags.set_pic_scaling_matrix_present_flag(self.p_scaling_lists.is_some() as u32);

        StdVideoH264PictureParameterSet {
            flags: pps_flags,
            seq_parameter_set_id: self.pps.seq_parameter_set_id.id(),
            pic_parameter_set_id: self.pps.pic_parameter_set_id.id(),
            num_ref_idx_l0_default_active_minus1: self.pps.num_ref_idx_l0_default_active_minus1 as u8,
            num_ref_idx_l1_default_active_minus1: self.pps.num_ref_idx_l1_default_active_minus1 as u8,
            weighted_bipred_idc: self.pps.weighted_bipred_idc as u32,
            pic_init_qp_minus26: self.pps.pic_init_qp_minus26 as i8,
            pic_init_qs_minus26: self.pps.pic_init_qs_minus26 as i8,
            chroma_qp_index_offset: self.pps.chroma_qp_index_offset as i8,
            second_chroma_qp_index_offset: self.pps.extension.as_ref().map_or(0, |ex| ex.second_chroma_qp_index_offset as i8),
            pScalingLists: self.p_scaling_lists.as_ref().map_or(null(), |p| p),
        }
    }
}
