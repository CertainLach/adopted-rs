use crate::{SessionId, TextPosition, TextSize};
use smallvec::SmallVec;
use std::ops::{Deref, DerefMut, RangeBounds};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Segment(SessionId, SmallVec<[u8; 16]>);
impl Deref for Segment {
    type Target = SmallVec<[u8; 16]>;

    fn deref(&self) -> &Self::Target {
        &self.1
    }
}
impl DerefMut for Segment {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.1
    }
}
impl Segment {
    #[inline]
    pub fn user(&self) -> SessionId {
        self.0
    }

    pub fn len(&self) -> TextSize {
        self.1.len() as TextSize
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SegmentBuffer {
    // Can be replaced with Vec<u8> and segments to (UserId, Range<usize>), instead of keeping every buffer inside of segment,
    // But it only would be faster for compaction, inserts would be slower
    segments: SmallVec<[Segment; 1]>,
    len: TextSize,
}
impl SegmentBuffer {
    pub fn new(segments: SmallVec<[Segment; 1]>) -> Self {
        let len = segments.iter().map(|s| s.len() as TextSize).sum();
        Self { segments, len }
    }
    pub fn compact(&mut self) {
        if self.segments.len() == 0 {
            return;
        }
        let mut last_segment = 0;
        let mut removed = Vec::new();
        loop {
            let (first, rest) = self.segments.split_at_mut(last_segment + 1);
            if rest.len() == 0 {
                break;
            }
            let first = &mut first[first.len() - 1];

            let end = rest
                .iter()
                .enumerate()
                .find(|(_, s)| s.user() != first.user())
                .map(|(i, _)| i)
                .unwrap_or(rest.len());

            let range = 0..end;
            removed.push(last_segment + 1..end + last_segment + 1);

            let to_move = rest[range.clone()].iter().map(|s| s.len()).sum();
            first.reserve_exact(to_move);

            for segment in rest[range].iter_mut() {
                first.extend(segment.drain(..));
            }

            last_segment = end + 1;

            if self.segments.len() == last_segment {
                break;
            }
        }
        for range in removed.into_iter().rev() {
            self.segments.drain(range);
        }
    }
    pub fn slice(&self, range: impl RangeBounds<TextPosition>) -> Self {
        let mut segments = SmallVec::new();
        let mut len: TextSize = 0;
        let mut start = match range.start_bound() {
            std::ops::Bound::Included(i) => *i,
            std::ops::Bound::Excluded(_) => unreachable!(),
            std::ops::Bound::Unbounded => 0,
        };
        let mut end = match range.end_bound() {
            std::ops::Bound::Included(i) => *i + 1,
            std::ops::Bound::Excluded(i) => *i,
            std::ops::Bound::Unbounded => self.len(),
        };
        if end > self.len() {
            panic!("slice out of range: {}", end)
        }
        for segment in self.segments.iter() {
            if start <= segment.len() {
                let end = segment.len().min(end);
                segments.push(Segment(segment.user(), segment[start..end].into()));
                len += end - start;
            }
            start = start.saturating_sub(segment.len());
            end = end.saturating_sub(segment.len());
            if end == 0 {
                break;
            }
        }
        Self { segments, len }
    }

    pub fn splice(&mut self, range: impl RangeBounds<usize>, insert: Option<SegmentBuffer>) {
        let mut start = match range.start_bound() {
            std::ops::Bound::Included(i) => *i,
            std::ops::Bound::Excluded(_) => unreachable!(),
            std::ops::Bound::Unbounded => 0,
        };
        let mut end = match range.end_bound() {
            std::ops::Bound::Included(i) => *i + 1,
            std::ops::Bound::Excluded(i) => *i,
            std::ops::Bound::Unbounded => self.len(),
        };
        if end > self.len() {
            panic!("splice out of range: {}", end)
        }
        let mut insert_at = None;
        let mut segment_idx = 0;
        while segment_idx < self.segments.len() {
            let segment_length = self.segments[segment_idx].len();
            if start < segment_length {
                println!("In segment {}: {}", segment_idx, start);
                let removed = start..end.min(segment_length);
                if start == 0 {
                    println!("Start");
                    // Beginning of segment
                    // abcdefg
                    // ^
                    if removed.end < segment_length {
                        // Start of segment
                        // abcdefg
                        // ^-^
                        if insert_at.is_none() {
                            insert_at = Some(segment_idx);
                        }
                        let old_segment = &self.segments[segment_idx];
                        let new_segment =
                            Segment(old_segment.user(), old_segment[removed.end..].into());
                        self.len -= removed.end;
                        self.segments[segment_idx] = new_segment;
                    } else {
                        // Full segment
                        // abcdefg
                        // ^-----^
                        self.len -= self.segments[segment_idx].len();
                        self.segments.remove(segment_idx);
                        if insert_at.is_none() {
                            insert_at = Some(segment_idx);
                        }
                        segment_idx -= 1;
                    }
                } else {
                    println!("Middle");
                    // Inside of segment
                    // abcdefg
                    //   ^
                    if insert_at.is_none() {
                        insert_at = Some(segment_idx + 1);
                    }
                    if removed.end < segment_length {
                        // Part of segment
                        // abcdefg
                        //   ^-^
                        let old_segment = &mut self.segments[segment_idx];
                        let new_segment =
                            Segment(old_segment.user(), old_segment[removed.end..].into());
                        old_segment.truncate(removed.start);
                        self.len -= removed.end - removed.start;
                        self.segments.insert(segment_idx + 1, new_segment);
                        segment_idx += 1;
                    } else {
                        // End of segment
                        // abcdefg
                        //   ^---^
                        self.segments[segment_idx].truncate(removed.start);
                        self.len -= removed.end - removed.start;
                    }
                }
                dbg!(end);
            }
            if start < segment_length && end == start {
                if insert_at.is_none() {
                    insert_at = Some(segment_idx);
                }
                break;
            }
            end = end.saturating_sub(segment_length);
            start = start.saturating_sub(segment_length);
            segment_idx += 1;
        }
        if let Some(insert) = insert {
            self.len += insert.len();
            let insert_at = insert_at.unwrap_or(self.segments.len());
            self.segments.insert_many(insert_at, insert.segments);
        }
        self.compact()
    }

    pub fn len(&self) -> TextSize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

#[cfg(test)]
mod tests {
    mod compact {
        use crate::segment::{Segment, SegmentBuffer};
        use smallvec::smallvec;

        #[test]
        fn simple() {
            let mut buf = SegmentBuffer::new(smallvec![
                Segment(1, smallvec![1, 2]),
                Segment(1, smallvec![3, 4]),
            ]);
            buf.compact();
            assert_eq!(
                buf,
                SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3, 4])])
            );
        }

        #[test]
        fn single() {
            let mut buf = SegmentBuffer::new(smallvec![
                Segment(1, smallvec![1, 2]),
                Segment(1, smallvec![3, 4]),
                Segment(2, smallvec![5]),
            ]);
            buf.compact();
            assert_eq!(
                buf,
                SegmentBuffer::new(smallvec![
                    Segment(1, smallvec![1, 2, 3, 4]),
                    Segment(2, smallvec![5])
                ])
            );
        }
    }

    mod slice {
        use crate::segment::{Segment, SegmentBuffer};
        use smallvec::smallvec;

        #[test]
        fn first() {
            let input = SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3, 4])]);
            assert_eq!(input.slice(0..=3), input);

            let input = SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3, 4])]);
            assert_eq!(input.slice(0..4), input);
        }

        #[test]
        fn part() {
            let input = SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3, 4])]);
            assert_eq!(
                input.slice(0..=2),
                SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3])])
            );

            let input = SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3, 4])]);
            assert_eq!(
                input.slice(1..=3),
                SegmentBuffer::new(smallvec![Segment(1, smallvec![2, 3, 4])])
            );
        }

        #[test]
        fn two() {
            let input = SegmentBuffer::new(smallvec![
                Segment(1, smallvec![1, 2, 3, 4]),
                Segment(1, smallvec![5, 6, 7, 8])
            ]);
            assert_eq!(
                input.slice(2..=5),
                SegmentBuffer::new(smallvec![
                    Segment(1, smallvec![3, 4]),
                    Segment(1, smallvec![5, 6])
                ])
            );

            let input = SegmentBuffer::new(smallvec![
                Segment(1, smallvec![1, 2, 3, 4]),
                Segment(1, smallvec![5, 6, 7, 8])
            ]);
            assert_eq!(input.slice(0..=7), input);
        }
    }

    mod splice {
        use crate::segment::{Segment, SegmentBuffer};
        use smallvec::smallvec;

        #[test]
        fn insert_start() {
            let mut buf = SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3, 4])]);
            buf.splice(
                0..0,
                Some(SegmentBuffer::new(smallvec![Segment(2, smallvec![5])])),
            );
            assert_eq!(
                buf,
                SegmentBuffer::new(smallvec![
                    Segment(2, smallvec![5]),
                    Segment(1, smallvec![1, 2, 3, 4])
                ])
            )
        }

        #[test]
        fn insert_end() {
            let mut buf = SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3, 4])]);
            buf.splice(
                buf.len..buf.len,
                Some(SegmentBuffer::new(smallvec![Segment(2, smallvec![5])])),
            );
            assert_eq!(
                buf,
                SegmentBuffer::new(smallvec![
                    Segment(1, smallvec![1, 2, 3, 4]),
                    Segment(2, smallvec![5]),
                ])
            )
        }

        #[test]
        fn insert_middle() {
            let mut buf = SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3, 4])]);
            buf.splice(
                2..2,
                Some(SegmentBuffer::new(smallvec![Segment(2, smallvec![5])])),
            );
            assert_eq!(
                buf,
                SegmentBuffer::new(smallvec![
                    Segment(1, smallvec![1, 2]),
                    Segment(2, smallvec![5]),
                    Segment(1, smallvec![3, 4]),
                ])
            )
        }

        #[test]
        fn replace_middle() {
            let mut buf = SegmentBuffer::new(smallvec![Segment(1, smallvec![1, 2, 3, 4])]);
            buf.splice(
                2..=2,
                Some(SegmentBuffer::new(smallvec![Segment(2, smallvec![5])])),
            );
            assert_eq!(
                buf,
                SegmentBuffer::new(smallvec![
                    Segment(1, smallvec![1, 2]),
                    Segment(2, smallvec![5]),
                    Segment(1, smallvec![4]),
                ])
            )
        }

        #[test]
        fn replace_middle_overlap() {
            let mut buf = SegmentBuffer::new(smallvec![
                Segment(1, smallvec![1, 2]),
                Segment(1, smallvec![3, 4])
            ]);
            buf.splice(
                1..3,
                Some(SegmentBuffer::new(smallvec![Segment(2, smallvec![5])])),
            );
            assert_eq!(
                buf,
                SegmentBuffer::new(smallvec![
                    Segment(1, smallvec![1]),
                    Segment(2, smallvec![5]),
                    Segment(1, smallvec![4]),
                ])
            )
        }
    }
}
