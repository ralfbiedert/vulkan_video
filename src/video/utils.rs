use core::iter::Enumerate;
use core::slice::Iter as SliceIter;
use h264_reader::nal::RefNal;

// How many `0` we have to observe before a `1` means NAL.
const NAL_MIN_0_COUNT: usize = 2;
/// Advances the given iterator until it finds the index of the next NAL start.
#[inline]
fn next_offset<'a>(iter: &mut Enumerate<SliceIter<'a, u8>>) -> Option<usize> {
    let mut count_0 = 0;
    for (offset, byte) in iter {
        match byte {
            0 => count_0 += 1,
            1 if count_0 >= NAL_MIN_0_COUNT => return Some(offset + 1),
            _ => count_0 = 0,
        }
    }
    None
}

/// Splits a bitstream into NAL units.
///
/// This function is useful if you happen to have a H.264 bitstream and want to decode it frame by frame: You
/// apply this function to the underlying stream and run your decoder on each returned slice, preferably
/// ignoring isolated decoding errors.
///
/// In detail, given a bitstream like so (`001` being the NAL start prefix code):
///
/// ```text
/// ......001.........001......001.....
/// ```
///
/// This function will return an iterator returning packets:
/// ```text
///      001[.......]001[....]001[.....]
/// ```
///
/// In other words, any incomplete data at the beginning of the buffer is skipped,
/// NAL units in the middle are split at their boundaries, the last packet is returned
/// as-is.
///
pub fn nal_units<'a>(stream: &'a [u8]) -> NalUnits<'a> {
    let mut iter = stream.into_iter().enumerate();
    let next_offset = next_offset(&mut iter);
    NalUnits { stream, iter, next_offset }
}
pub struct NalUnits<'a> {
    stream: &'a [u8],
    iter: Enumerate<SliceIter<'a, u8>>,
    next_offset: Option<usize>,
}
impl<'a> Iterator for NalUnits<'a> {
    type Item = RefNal<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.next_offset?;
        self.next_offset = next_offset(&mut self.iter);
        let nal = match self.next_offset {
            // Do not include 001 in nal buffer
            Some(next_offset) => &self.stream[offset..next_offset - (NAL_MIN_0_COUNT + 1)],
            None => &self.stream[offset..],
        };

        // required by RefNal::new
        if nal.is_empty() {
            None
        } else {
            Some(RefNal::new(nal, &[], true))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn splits_at_nal() {
        let stream = [];
        assert!(nal_units(&stream).next().is_none());

        let stream = [2, 3];
        assert!(nal_units(&stream).next().is_none());

        let stream = [0, 0, 1];
        // nal unit is "empty", so no RefNal is produced
        assert!(nal_units(&stream).next().is_none());

        let stream = [0, 0, 1, 2];
        assert_eq!(nal_units(&stream).next().unwrap(), RefNal::new(&[2], &[], true));

        let stream = [0, 0, 1, 2, 0, 0, 1];
        let mut split = nal_units(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[2], &[], true));
        assert!(split.next().is_none());

        let stream = [0, 0, 0, 0, 0, 1, 2, 0, 0, 1];
        let mut split = nal_units(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[2], &[], true));
        assert!(split.next().is_none());

        let stream = [0, 0, 0, 0, 0, 1, 2, 0, 0];
        let mut split = nal_units(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[2, 0, 0], &[], true));
        assert!(split.next().is_none());

        let stream = [0, 0, 0, 0, 0, 1, 2, 0, 0, 1, 2, 3, 0, 0, 1];
        let mut split = nal_units(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[2], &[], true));
        assert_eq!(split.next().unwrap(), RefNal::new(&[2, 3], &[], true));
        assert!(split.next().is_none());
    }
}
