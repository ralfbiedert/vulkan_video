use crate::Error;
use ash::vk::native::{
    StdVideoH264PictureParameterSet, StdVideoH264ScalingLists, StdVideoH264SequenceParameterSet, StdVideoH264SequenceParameterSetVui,
    StdVideoH264SpsFlags, StdVideoH264SpsVuiFlags,
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
struct SpsInfo<'a> {
    sps: &'a SeqParameterSet,
    p_offset_for_ref_frame: Option<i32>,
    p_scaling_lists: Option<StdVideoH264ScalingLists>,
    p_sequence_parameter_set_vui: Option<StdVideoH264SequenceParameterSetVui>,
}
impl<'a> SpsInfo<'a> {
    fn new(sps: &'a SeqParameterSet) -> Self {
        // let p_offset_for_ref_frame=0;
        // let p_scaling_lists=sps.chroma_info.scaling_matrix.as_ref().map(|matrix|{
        // 	matrix.scaling_list4x4
        // 	StdVideoH264ScalingLists
        // });
        let p_sequence_parameter_set_vui = sps.vui_parameters.as_ref().map(|vui| {
            let mut flags = StdVideoH264SpsVuiFlags {
                _bitfield_align_1: [],
                _bitfield_1: Default::default(),
                __bindgen_padding_0: 0,
            };

            StdVideoH264SequenceParameterSetVui {
                flags,
                aspect_ratio_idc: (),
                sar_width: (),
                sar_height: (),
                video_format: (),
                colour_primaries: (),
                transfer_characteristics: (),
                matrix_coefficients: (),
                num_units_in_tick: (),
                time_scale: (),
                max_num_reorder_frames: (),
                max_dec_frame_buffering: (),
                chroma_sample_loc_type_top_field: (),
                chroma_sample_loc_type_bottom_field: (),
                reserved1: (),
                pHrdParameters: (),
            }
        });
        SpsInfo {
            sps,
            p_offset_for_ref_frame,
            p_scaling_lists,
            p_sequence_parameter_set_vui,
        }
    }
}
pub struct SpsStep1<'a> {
    sps_infos: Vec<SpsInfo<'a>>,
}
impl SpsStep1<'_> {
    fn new<'a>(h264_context: &'a Context) -> SpsStep1<'a> {
        SpsStep1 {
            sps_infos: h264_context.sps().map(SpsInfo::new).collect(),
        }
    }
    pub fn step2<'a>(&'a self) -> SpsStep2<'a> {
        let sps = self
            .sps_infos
            .iter()
            .map(|sps_info| {
                let frame_crop_left_offset = sps_info.sps.frame_cropping.map(|f| f.left_offset).unwrap_or(0);
                let frame_crop_right_offset = sps_info.sps.frame_cropping.map(|f| f.right_offset).unwrap_or(0);
                let frame_crop_top_offset = sps_info.sps.frame_cropping.map(|f| f.top_offset).unwrap_or(0);
                let frame_crop_bottom_offset = sps_info.sps.frame_cropping.map(|f| f.bottom_offset).unwrap_or(0);
                let mut flags = StdVideoH264SpsFlags {
                    _bitfield_align_1: [],
                    _bitfield_1: Default::default(),
                    __bindgen_padding_0: 0,
                };
                flags.set_constraint_set0_flag(sps_info.sps.constraint_flags.flag0() as u32);
                flags.set_constraint_set1_flag(sps_info.sps.constraint_flags.flag1() as u32);
                flags.set_constraint_set2_flag(sps_info.sps.constraint_flags.flag2() as u32);
                flags.set_constraint_set3_flag(sps_info.sps.constraint_flags.flag3() as u32);
                flags.set_constraint_set4_flag(sps_info.sps.constraint_flags.flag4() as u32);
                flags.set_constraint_set5_flag(sps_info.sps.constraint_flags.flag5() as u32);
                flags.set_vui_parameters_present_flag(sps_info.p_sequence_parameter_set_vui.is_some() as u32);
                StdVideoH264SequenceParameterSet {
                    profile_idc: sps_info.sps.profile_idc.to_u8(),
                    level_idc: sps_info.sps.level_idc as u32,
                    seq_parameter_set_id: sps_info.sps.seq_parameter_set_id.id(),
                    log2_max_frame_num_minus4: sps_info.sps.log2_max_frame_num_minus4,
                    max_num_ref_frames: sps_info.sps.max_num_ref_frames as u8,
                    pic_width_in_mbs_minus1: sps_info.sps.pic_width_in_mbs_minus1,
                    pic_height_in_map_units_minus1: sps_info.sps.pic_height_in_map_units_minus1,
                    chroma_format_idc: sps_info.sps.chroma_info.chroma_format.id(),
                    bit_depth_luma_minus8: sps_info.sps.chroma_info.bit_depth_luma_minus8,
                    bit_depth_chroma_minus8: sps_info.sps.chroma_info.bit_depth_chroma_minus8,
                    pic_order_cnt_type: sps_info.sps.pic_order_cnt.id(),
                    flags,
                    frame_crop_left_offset,
                    frame_crop_right_offset,
                    frame_crop_top_offset,
                    frame_crop_bottom_offset,
                    // gaps_in_frame_num_value_allowed_flag: sps.gaps_in_frame_num_value_allowed_flag,
                    // frame_mbs_flags: sps.frame_mbs_flags,
                    // direct_8x8_inference_flag: sps.direct_8x8_inference_flag,
                    // vui_parameters: sps.vui_parameters,
                    offset_for_non_ref_pic: 0,
                    offset_for_top_to_bottom_field: 0,
                    log2_max_pic_order_cnt_lsb_minus4: 0,
                    num_ref_frames_in_pic_order_cnt_cycle: 0,
                    pOffsetForRefFrame: sps_info.p_offset_for_ref_frame.as_ref().map_or(null(), |p| p),
                    pScalingLists: sps_info.p_scaling_lists.as_ref().map_or(null(), |p| p),
                    pSequenceParameterSetVui: sps_info.p_sequence_parameter_set_vui.as_ref().map_or(null(), |p| p),
                    reserved1: 0,
                    reserved2: 0,
                }
            })
            .collect();
        SpsStep2 { sps, _marker: PhantomData }
    }
}
pub struct SpsStep2<'a> {
    sps: Vec<StdVideoH264SequenceParameterSet>,
    _marker: PhantomData<&'a ()>,
}
impl SpsStep2<'_> {
    pub fn array<'a>(&'a self) -> &'a [StdVideoH264SequenceParameterSet] {
        &self.sps
    }
}

struct PpsInfo<'a> {
    pps: &'a PicParameterSet,
    p_scaling_lists: Option<StdVideoH264ScalingLists>,
}
impl<'a> PpsInfo<'a> {
    fn new(pps: &'a PicParameterSet) -> Self {
        // let p_offset_for_ref_frame=0;
        // let p_scaling_lists=pps.chroma_info.scaling_matrix.as_ref().map(|matrix|{
        // 	matrix.scaling_list4x4
        // 	StdVideoH264ScalingLists
        // });
        PpsInfo { pps, p_scaling_lists }
    }
}
pub struct PpsStep1<'a> {
    pps_infos: Vec<PpsInfo<'a>>,
}
impl PpsStep1<'_> {
    fn new<'a>(h264_context: &'a Context) -> PpsStep1<'a> {
        PpsStep1 {
            pps_infos: h264_context.pps().map(PpsInfo::new).collect(),
        }
    }
    pub fn step2<'a>(&'a self) -> PpsStep2<'a> {
        let pps = self
            .pps_infos
            .iter()
            .map(|pps_info| StdVideoH264PictureParameterSet {
                flags,
                seq_parameter_set_id: todo!(),
                pic_parameter_set_id: todo!(),
                num_ref_idx_l0_default_active_minus1: todo!(),
                num_ref_idx_l1_default_active_minus1: todo!(),
                weighted_bipred_idc: todo!(),
                pic_init_qp_minus26: todo!(),
                pic_init_qs_minus26: todo!(),
                chroma_qp_index_offset: todo!(),
                second_chroma_qp_index_offset: todo!(),
                pScalingLists: todo!(),
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
