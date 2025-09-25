use h264_reader::nal::RefNal;

// How many `0` we have to observe before a `1` means NAL.
const NAL_MIN_0_COUNT: usize = 2;

/// Given a stream, finds the index of the nth NAL start.
#[inline]
fn nth_nal_index(stream: &[u8], nth: usize) -> Option<usize> {
    let mut count_0 = 0;
    let mut n = 0;

    for (i, byte) in stream.iter().enumerate() {
        match byte {
            0 => count_0 += 1,
            1 if count_0 >= NAL_MIN_0_COUNT => {
                if n == nth {
                    return Some(i - NAL_MIN_0_COUNT);
                } else {
                    count_0 = 0;
                    n += 1;
                }
            }
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
pub fn nal_units<'a>(mut stream: &'a [u8]) -> impl Iterator<Item = RefNal<'a>> {
    std::iter::from_fn(move || {
        let first = nth_nal_index(stream, 0);
        let next = nth_nal_index(stream, 1);

        match (first, next) {
            (Some(f), Some(n)) => {
                let rval = &stream[f..n];
                stream = &stream[n..];
                Some(RefNal::new(rval, &[], true))
            }
            (Some(f), None) => {
                let rval = &stream[f..];
                stream = &stream[f + NAL_MIN_0_COUNT..];
                Some(RefNal::new(rval, &[], true))
            }
            _ => None,
        }
    })
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
        assert_eq!(nal_units(&stream).next().unwrap(), RefNal::new(&[0, 0, 1], &[], true));

        let stream = [0, 0, 1, 2];
        assert_eq!(nal_units(&stream).next().unwrap(), RefNal::new(&[0, 0, 1, 2], &[], true));

        let stream = [0, 0, 1, 2, 0, 0, 1];
        let mut split = nal_units(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2], &[], true));
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1], &[], true));
        assert!(split.next().is_none());

        let stream = [0, 0, 0, 0, 0, 1, 2, 0, 0, 1];
        let mut split = nal_units(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2], &[], true));
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1], &[], true));
        assert!(split.next().is_none());

        let stream = [0, 0, 0, 0, 0, 1, 2, 0, 0];
        let mut split = nal_units(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2, 0, 0], &[], true));
        assert!(split.next().is_none());

        let stream = [0, 0, 0, 0, 0, 1, 2, 0, 0, 1, 2, 3, 0, 0, 1];
        let mut split = nal_units(&stream);
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2], &[], true));
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1, 2, 3], &[], true));
        assert_eq!(split.next().unwrap(), RefNal::new(&[0, 0, 1], &[], true));
        assert!(split.next().is_none());
    }
}
