use h264_reader::nal::RefNal;

// How many `0` we have to observe before a `1` means NAL.
const NAL_MIN_0_COUNT: usize = 2;

/// Given a stream, finds the index of the nth NAL start.
#[inline]
fn next_offset<'a>(iter: &mut core::iter::Enumerate<core::slice::Iter<'a, u8>>) -> Option<usize> {
    let mut count_0 = 0;
    for (offset, byte) in iter {
        match byte {
            0 => count_0 += 1,
            1 if count_0 >= NAL_MIN_0_COUNT => return Some(offset - NAL_MIN_0_COUNT),
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
///      [001.......][001....][001.....]
/// ```
///
/// In other words, any incomplete data at the beginning of the buffer is skipped,
/// NAL units in the middle are split at their boundaries, the last packet is returned
/// as-is.
///
pub struct NalIter<'a> {
    stream: &'a [u8],
    iter: core::iter::Enumerate<core::slice::Iter<'a, u8>>,
    state: NalIterState,
}
enum NalIterState {
    Next { start: usize, end: usize },
    Last { start: usize },
    End,
}
impl<'a> NalIter<'a> {
    pub fn new(stream: &'a [u8]) -> Self {
        let mut iter = stream.into_iter().enumerate();
        let state = match next_offset(&mut iter) {
            Some(offset) => match next_offset(&mut iter) {
                Some(next_offset) => NalIterState::Next {
                    start: offset,
                    end: next_offset,
                },
                None => NalIterState::Last { start: offset },
            },
            None => NalIterState::End,
        };
        Self { stream, iter, state }
    }
}
impl<'a> Iterator for NalIter<'a> {
    type Item = RefNal<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            NalIterState::Next { start, end } => {
                self.state = match next_offset(&mut self.iter) {
                    Some(next_offset) => NalIterState::Next {
                        start: end,
                        end: next_offset,
                    },
                    None => NalIterState::Last { start: end },
                };
                Some(RefNal::new(&self.stream[start..end], &[], true))
            }
            NalIterState::Last { start } => {
                self.state = NalIterState::End;
                Some(RefNal::new(&self.stream[start..], &[], true))
            }
            NalIterState::End => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn splits_at_nal() {
        let stream = [];
        assert!(NalIter::new(&stream).next().is_none());

        let stream = [2, 3];
        assert!(NalIter::new(&stream).next().is_none());

        let stream = [0, 0, 1];
        assert_eq!(NalIter::new(&stream).next().unwrap(), RefNal::new(&[0, 0, 1], &[], true));

        let stream = [0, 0, 1, 2];
        assert_eq!(NalIter::new(&stream).next().unwrap(), RefNal::new(&[0, 0, 1, 2], &[], true));

        let stream = [0, 0, 1, 2, 0, 0, 1];
        let mut split = NalIter::new(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2], &[], true));
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1], &[], true));
        assert!(split.next().is_none());

        let stream = [0, 0, 0, 0, 0, 1, 2, 0, 0, 1];
        let mut split = NalIter::new(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2], &[], true));
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1], &[], true));
        assert!(split.next().is_none());

        let stream = [0, 0, 0, 0, 0, 1, 2, 0, 0];
        let mut split = NalIter::new(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2, 0, 0], &[], true));
        assert!(split.next().is_none());

        let stream = [0, 0, 0, 0, 0, 1, 2, 0, 0, 1, 2, 3, 0, 0, 1];
        let mut split = NalIter::new(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2], &[], true));
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2, 3], &[], true));
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1], &[], true));
        assert!(split.next().is_none());
    }
}
