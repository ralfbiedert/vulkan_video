use h264_reader::nal::pps::PicParameterSet;
use h264_reader::nal::sps::SeqParameterSet;
use h264_reader::nal::{Nal, RefNal, UnitType};
use h264_reader::Context;
use vulkan_video::video::nal_units;

#[test]
fn parse_h264_nals() {
    let h264_data = include_bytes!("videos/multi_512x512.h264");

    let mut context = Context::new();

    let mut feed_nal = |nal: RefNal<'_>| {
        let nal_unit_type = nal.header().unwrap().nal_unit_type();
        let bits = nal.rbsp_bits();

        match nal_unit_type {
            UnitType::SeqParameterSet => {
                let sps = SeqParameterSet::from_bits(bits).unwrap();
                assert_eq!(sps.level_idc, 31);
                context.put_seq_param_set(sps);
            }
            UnitType::PicParameterSet => {
                let pps = PicParameterSet::from_bits(&context, bits).unwrap();
                assert_eq!(pps.pic_init_qp_minus26, -6);
            }
            _ => {}
        }
    };

    // Push a couple NALs. Pushes don't have to match up to Annex B framing.
    for nal in nal_units(h264_data) {
        feed_nal(nal);
    }
}
