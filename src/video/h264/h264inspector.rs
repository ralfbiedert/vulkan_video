use crate::Error;
use ash::vk::native::{
    StdVideoH264HrdParameters, StdVideoH264PictureParameterSet, StdVideoH264PpsFlags, StdVideoH264ScalingLists,
    StdVideoH264SequenceParameterSet, StdVideoH264SequenceParameterSetVui, StdVideoH264SpsFlags, StdVideoH264SpsVuiFlags,
};
use ash::vk::{
    VideoChromaSubsamplingFlagsKHR, VideoCodecOperationFlagsKHR, VideoComponentBitDepthFlagsKHR, VideoDecodeH264PictureLayoutFlagsKHR,
    VideoDecodeH264ProfileInfoKHR, VideoProfileInfoKHR,
};
use core::marker::PhantomData;
use core::ptr::null;
use h264_reader::annexb::AnnexBReader;
use h264_reader::nal::pps::{PicParameterSet, PpsError};
use h264_reader::nal::sps::{SeqParameterSet, SpsError};
use h264_reader::nal::{Nal, NalHeader, NalHeaderError, RefNal, UnitType};
use h264_reader::push::{NalFragmentHandler, NalInterest};
use h264_reader::Context;

/// Parses H.264 NAL units and returns mata data we need to feed into Vulkan.
#[derive(Default)]
pub struct H264StreamInspector {
    h264_context: Context,
}

#[derive(Debug)]
pub enum FeedError {
    NalHeader(NalHeaderError),
    Pps(PpsError),
    Sps(SpsError),
}

impl H264StreamInspector {
    pub fn new() -> Self {
        Self {
            h264_context: Default::default(),
        }
    }

    pub fn feed_nal(&mut self, nal: RefNal<'_>) -> Result<(), FeedError> {
        let nal_unit_type = nal.header().map_err(FeedError::NalHeader)?.nal_unit_type();
        let bits = nal.rbsp_bits();

        match nal_unit_type {
            UnitType::SeqParameterSet => {
                let sps = SeqParameterSet::from_bits(bits).map_err(FeedError::Sps)?;
                self.h264_context.put_seq_param_set(sps);
            }
            UnitType::PicParameterSet => {
                let pps = PicParameterSet::from_bits(&self.h264_context, bits).map_err(FeedError::Pps)?;
                self.h264_context.put_pic_param_set(pps);
            }
            _ => {}
        }

        Ok(())
    }

    pub fn h264_profile_info<'a>(&self) -> VideoDecodeH264ProfileInfoKHR<'a> {
        VideoDecodeH264ProfileInfoKHR::default()
            .picture_layout(VideoDecodeH264PictureLayoutFlagsKHR::INTERLACED_INTERLEAVED_LINES)
            .std_profile_idc(100)
    }
    pub fn profile_info<'a>(&self, h264_profile_info: &'a mut VideoDecodeH264ProfileInfoKHR<'_>) -> VideoProfileInfoKHR<'a> {
        VideoProfileInfoKHR::default()
            .push_next(h264_profile_info)
            .video_codec_operation(VideoCodecOperationFlagsKHR::DECODE_H264)
            .chroma_subsampling(VideoChromaSubsamplingFlagsKHR::TYPE_420)
            .luma_bit_depth(VideoComponentBitDepthFlagsKHR::TYPE_8)
            .chroma_bit_depth(VideoComponentBitDepthFlagsKHR::TYPE_8)
    }

    pub fn h264_sps<'a>(&'a self) -> SpsStep1<'a> {
        SpsStep1::new(&self.h264_context)
    }
    pub fn h264_pps<'a>(&'a self) -> PpsStep1<'a> {
        PpsStep1::new(&self.h264_context)
    }
}
struct SpsInfo1<'a> {
    sps: &'a SeqParameterSet,
    p_offset_for_ref_frame: Option<i32>,
    p_scaling_lists: Option<StdVideoH264ScalingLists>,
    p_hrd_parameters: Option<StdVideoH264HrdParameters>,
}
impl<'a> SpsInfo1<'a> {
    fn new(sps: &'a SeqParameterSet) -> Self {
        let p_offset_for_ref_frame = None;
        let p_scaling_lists = None;
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
        SpsInfo1 {
            sps,
            p_offset_for_ref_frame,
            p_scaling_lists,
            p_hrd_parameters,
        }
    }
    fn step2<'b>(&'b self) -> SpsInfo2<'b> {
        let p_sequence_parameter_set_vui = self.sps.vui_parameters.as_ref().map(|vui| {
            let mut flags = StdVideoH264SpsVuiFlags {
                _bitfield_align_1: [],
                _bitfield_1: Default::default(),
                __bindgen_padding_0: 0,
            };
            // let mut aspect_ratio_idc = 0;
            // if let Some(aspect_ratio_info) = &vui.aspect_ratio_info {
            //     flags.set_aspect_ratio_info_present_flag(1);
            //     aspect_ratio_idc = aspect_ratio_info.get().map_or(0, |(x, y)| ((y as u32) << 16) + (x as u32));
            // }
            // // flags.set_overscan_info_present_flag(vui.overscan_info_present);
            // flags.set_overscan_appropriate_flag(
            //     (vui.overscan_appropriate == h264_reader::nal::sps::OverscanAppropriate::Appropriate) as u32,
            // );
            // flags.set_video_signal_type_present_flag(vui.video_signal_type.is_some() as u32);
            // // flags.set_video_full_range_flag(vui.video_full_range);
            // // flags.set_color_description_present_flag(vui.color_description_present);
            // flags.set_chroma_loc_info_present_flag(vui.chroma_loc_info.is_some() as u32);
            // flags.set_timing_info_present_flag(vui.timing_info.is_some() as u32);
            // // flags.set_fixed_frame_rate_flag(vui.fixed_frame_rate);
            // flags.set_bitstream_restriction_flag(vui.bitstream_restrictions.is_some() as u32);
            // flags.set_nal_hrd_parameters_present_flag(vui.nal_hrd_parameters.is_some() as u32);
            // flags.set_vcl_hrd_parameters_present_flag(vui.vcl_hrd_parameters.is_some() as u32);

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
            info1: self,
            p_sequence_parameter_set_vui,
        }
    }
}
pub struct SpsStep1<'a> {
    sps_infos: Vec<SpsInfo1<'a>>,
}
impl SpsStep1<'_> {
    fn new<'a>(h264_context: &'a Context) -> SpsStep1<'a> {
        SpsStep1 {
            sps_infos: h264_context.sps().map(SpsInfo1::new).collect(),
        }
    }
    pub fn step2<'a>(&'a self) -> SpsStep2<'a> {
        SpsStep2 {
            sps_infos: self.sps_infos.iter().map(|sps_info| sps_info.step2()).collect(),
        }
    }
}

struct SpsInfo2<'a> {
    info1: &'a SpsInfo1<'a>,
    p_sequence_parameter_set_vui: Option<StdVideoH264SequenceParameterSetVui>,
}
pub struct SpsStep2<'a> {
    sps_infos: Vec<SpsInfo2<'a>>,
}
impl SpsStep2<'_> {
    pub fn step3<'a>(&'a self) -> SpsStep3<'a> {
        let sps = self
            .sps_infos
            .iter()
            .map(|sps_info2| {
                let sps_info1 = sps_info2.info1;
                // let frame_crop_left_offset = sps_info1.sps.frame_cropping.map(|f| f.left_offset).unwrap_or(0);
                // let frame_crop_right_offset = sps_info1.sps.frame_cropping.map(|f| f.right_offset).unwrap_or(0);
                // let frame_crop_top_offset = sps_info1.sps.frame_cropping.map(|f| f.top_offset).unwrap_or(0);
                // let frame_crop_bottom_offset = sps_info1.sps.frame_cropping.map(|f| f.bottom_offset).unwrap_or(0);
                // let mut flags = StdVideoH264SpsFlags {
                //     _bitfield_align_1: [],
                //     _bitfield_1: Default::default(),
                //     __bindgen_padding_0: 0,
                // };
                // flags.set_constraint_set0_flag(sps_info1.sps.constraint_flags.flag0() as u32);
                // flags.set_constraint_set1_flag(sps_info1.sps.constraint_flags.flag1() as u32);
                // flags.set_constraint_set2_flag(sps_info1.sps.constraint_flags.flag2() as u32);
                // flags.set_constraint_set3_flag(sps_info1.sps.constraint_flags.flag3() as u32);
                // flags.set_constraint_set4_flag(sps_info1.sps.constraint_flags.flag4() as u32);
                // flags.set_constraint_set5_flag(sps_info1.sps.constraint_flags.flag5() as u32);
                // flags.set_vui_parameters_present_flag(sps_info2.p_sequence_parameter_set_vui.is_some() as u32);
                // StdVideoH264SequenceParameterSet {
                //     profile_idc: sps_info1.sps.profile_idc.to_u8(),
                //     level_idc: sps_info1.sps.level_idc as u32,
                //     seq_parameter_set_id: sps_info1.sps.seq_parameter_set_id.id(),
                //     log2_max_frame_num_minus4: sps_info1.sps.log2_max_frame_num_minus4,
                //     max_num_ref_frames: sps_info1.sps.max_num_ref_frames as u8,
                //     pic_width_in_mbs_minus1: sps_info1.sps.pic_width_in_mbs_minus1,
                //     pic_height_in_map_units_minus1: sps_info1.sps.pic_height_in_map_units_minus1,
                //     chroma_format_idc: sps_info1.sps.chroma_info.chroma_format.id(),
                //     bit_depth_luma_minus8: sps_info1.sps.chroma_info.bit_depth_luma_minus8,
                //     bit_depth_chroma_minus8: sps_info1.sps.chroma_info.bit_depth_chroma_minus8,
                //     pic_order_cnt_type: sps_info1.sps.pic_order_cnt.id(),
                //     flags,
                //     frame_crop_left_offset,
                //     frame_crop_right_offset,
                //     frame_crop_top_offset,
                //     frame_crop_bottom_offset,
                //     // gaps_in_frame_num_value_allowed_flag: sps.gaps_in_frame_num_value_allowed_flag,
                //     // frame_mbs_flags: sps.frame_mbs_flags,
                //     // direct_8x8_inference_flag: sps.direct_8x8_inference_flag,
                //     // vui_parameters: sps.vui_parameters,
                //     offset_for_non_ref_pic: 0,
                //     offset_for_top_to_bottom_field: 0,
                //     log2_max_pic_order_cnt_lsb_minus4: 0,
                //     num_ref_frames_in_pic_order_cnt_cycle: 0,
                //     pOffsetForRefFrame: sps_info1.p_offset_for_ref_frame.as_ref().map_or(null(), |p| p),
                //     pScalingLists: sps_info1.p_scaling_lists.as_ref().map_or(null(), |p| p),
                //     pSequenceParameterSetVui: sps_info1.p_sequence_parameter_set_vui.as_ref().map_or(null(), |p| p),
                //     reserved1: 0,
                //     reserved2: 0,
                // }
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
                    pOffsetForRefFrame: sps_info1.p_offset_for_ref_frame.as_ref().map_or(null(), |p| p),
                    pScalingLists: sps_info1.p_scaling_lists.as_ref().map_or(null(), |p| p),
                    pSequenceParameterSetVui: sps_info2.p_sequence_parameter_set_vui.as_ref().map_or(null(), |p| p),
                }
            })
            .collect();
        SpsStep3 { sps, _marker: PhantomData }
    }
}
pub struct SpsStep3<'a> {
    sps: Vec<StdVideoH264SequenceParameterSet>,
    _marker: PhantomData<&'a ()>,
}
impl SpsStep3<'_> {
    pub fn array<'a>(&'a self) -> &'a [StdVideoH264SequenceParameterSet] {
        &self.sps
    }
}

struct PpsInfo1<'a> {
    pps: &'a PicParameterSet,
    p_scaling_lists: Option<StdVideoH264ScalingLists>,
}
impl<'a> PpsInfo1<'a> {
    fn new(pps: &'a PicParameterSet) -> Self {
        let p_scaling_lists = None;
        // let p_scaling_lists = pps
        //     .extension
        //     .as_ref()
        //     .and_then(|extra| extra.pic_scaling_matrix.as_ref())
        //     .map(|matrix| StdVideoH264ScalingLists);
        PpsInfo1 { pps, p_scaling_lists }
    }
}
pub struct PpsStep1<'a> {
    pps_infos: Vec<PpsInfo1<'a>>,
}
impl PpsStep1<'_> {
    fn new<'a>(h264_context: &'a Context) -> PpsStep1<'a> {
        PpsStep1 {
            pps_infos: h264_context.pps().map(PpsInfo1::new).collect(),
        }
    }
    pub fn step2<'a>(&'a self) -> PpsStep2<'a> {
        let pps = self
            .pps_infos
            .iter()
            .map(|pps_info| {
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
                    pScalingLists: pps_info.p_scaling_lists.as_ref().map_or(null(), |p| p),
                }
            })
            .collect();
        PpsStep2 { pps, _marker: PhantomData }
    }
}
pub struct PpsStep2<'a> {
    pps: Vec<StdVideoH264PictureParameterSet>,
    _marker: PhantomData<&'a ()>,
}
impl PpsStep2<'_> {
    pub fn array<'a>(&'a self) -> &'a [StdVideoH264PictureParameterSet] {
        &self.pps
    }
}

#[cfg(test)]
mod test {
    use crate::error::Error;
    use crate::video::h264::H264StreamInspector;
    use crate::video::nal_units;
    use ash::vk::{VideoCodecOperationFlagsKHR, VideoProfileListInfoKHR};

    #[test]
    fn get_profile_info_list() -> Result<(), Error> {
        let inspector = H264StreamInspector::new();
        let mut h264_profile_info = inspector.h264_profile_info();
        let profiles = &[inspector.profile_info(&mut h264_profile_info)];
        let infos = VideoProfileListInfoKHR::default().profiles(profiles);

        unsafe {
            assert_eq!(infos.profile_count, 1);
            assert_eq!((*infos.p_profiles).video_codec_operation, VideoCodecOperationFlagsKHR::DECODE_H264);
        }

        Ok(())
    }

    #[test]
    fn inspect_h264_stream() -> Result<(), Error> {
        let h264_data = include_bytes!("../../../tests/videos/multi_512x512.h264");

        let mut inspector = H264StreamInspector::new();

        // Push a couple NALs. Pushes don't have to match up to Annex B framing.
        for nal in nal_units(h264_data) {
            inspector.feed_nal(nal).unwrap();
        }

        Ok(())
    }
}
