use crate::Error;
use ash::vk::{
    VideoChromaSubsamplingFlagsKHR, VideoCodecOperationFlagsKHR, VideoComponentBitDepthFlagsKHR, VideoDecodeH264PictureLayoutFlagsKHR,
    VideoDecodeH264ProfileInfoKHR, VideoProfileInfoKHR, VideoProfileListInfoKHR,
};
use h264_reader::annexb::AnnexBReader;
use h264_reader::nal::pps::PicParameterSet;
use h264_reader::nal::sps::SeqParameterSet;
use h264_reader::nal::{Nal, NalHeader, NalHeaderError, RefNal, UnitType};
use h264_reader::push::{NalFragmentHandler, NalInterest};
use h264_reader::Context;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::ptr::addr_of;

#[derive(Default)]
pub struct VideoProfileInfoBundle<'a> {
    pub(crate) info_h264: VideoDecodeH264ProfileInfoKHR<'a>,
    pub(crate) info: VideoProfileInfoKHR<'a>,
    pub(crate) list: VideoProfileListInfoKHR<'a>,
    _pinned: PhantomPinned,
}

/// Parses H.264 NAL units and returns mata data we need to feed into Vulkan.
#[derive(Default)]
pub struct H264StreamInspector {
    h264_context: Context,
}

impl H264StreamInspector {
    pub fn new() -> Self {
        Self {
            h264_context: Default::default(),
        }
    }

    pub fn feed_nal(&mut self, nal: RefNal<'_>) {
        let nal_unit_type = nal.header().unwrap().nal_unit_type(); // TODO: Remove unwrap(), see above.
        let bits = nal.rbsp_bits();

        match nal_unit_type {
            UnitType::SeqParameterSet => {
                let sps = SeqParameterSet::from_bits(bits).unwrap(); // TODO: Remove unwrap(), see above.

                dbg!(&sps.chroma_info);

                self.h264_context.put_seq_param_set(sps);
            }
            UnitType::PicParameterSet => {
                // TODO: Remove unwrap(), see above.
                let _pps = PicParameterSet::from_bits(&self.h264_context, bits).unwrap();
            }
            _ => {} // _ => NalInterest::Ignore,
        }
    }

    pub fn profiles<'f>(&self) -> Pin<Box<VideoProfileInfoBundle<'f>>> {
        let mut inner = Box::pin(VideoProfileInfoBundle::default());

        let m = unsafe { inner.as_mut().get_unchecked_mut() };

        m.info_h264.picture_layout = VideoDecodeH264PictureLayoutFlagsKHR::INTERLACED_INTERLEAVED_LINES;
        m.info_h264.std_profile_idc = 100;

        m.info.p_next = addr_of!(m.info_h264).cast();
        m.info.video_codec_operation = VideoCodecOperationFlagsKHR::DECODE_H264;
        m.info.chroma_subsampling = VideoChromaSubsamplingFlagsKHR::TYPE_420;
        m.info.luma_bit_depth = VideoComponentBitDepthFlagsKHR::TYPE_8;
        m.info.chroma_bit_depth = VideoComponentBitDepthFlagsKHR::TYPE_8;

        m.list = VideoProfileListInfoKHR {
            p_profiles: addr_of!(m.info),
            profile_count: 1,
            ..Default::default()
        };

        inner
    }
}

#[cfg(test)]
mod test {
    use crate::error::Error;
    use crate::video::h264::H264StreamInspector;
    use crate::video::nal_units;
    use ash::vk::VideoCodecOperationFlagsKHR;

    #[test]
    fn get_profile_info_list() -> Result<(), Error> {
        let inspector = H264StreamInspector::new();
        let mut profiles = inspector.profiles();
        let infos = unsafe { &mut profiles.as_mut().get_unchecked_mut().list };

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
            inspector.feed_nal(nal);
        }

        Ok(())
    }
}
