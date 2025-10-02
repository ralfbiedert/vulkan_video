use crate::Error;
use ash::vk::{
    VideoChromaSubsamplingFlagsKHR, VideoCodecOperationFlagsKHR, VideoComponentBitDepthFlagsKHR, VideoDecodeH264PictureLayoutFlagsKHR,
    VideoDecodeH264ProfileInfoKHR, VideoProfileInfoKHR,
};
use h264_reader::annexb::AnnexBReader;
use h264_reader::nal::pps::{PicParameterSet, PpsError};
use h264_reader::nal::sps::{SeqParameterSet, SpsError};
use h264_reader::nal::{Nal, NalHeader, NalHeaderError, RefNal, UnitType};
use h264_reader::push::{NalFragmentHandler, NalInterest};
use h264_reader::Context;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::addr_of;

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
            .picture_layout(VideoDecodeH264PictureLayoutFlagsKHR::PROGRESSIVE)
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

        assert!(inspector.h264_context.sps().count() != 0);
        assert!(inspector.h264_context.pps().count() != 0);

        Ok(())
    }
}
