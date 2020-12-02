use crate::{segment::SegmentBuffer, TextPosition};
use std::ops::Deref;

#[derive(Clone, Debug)]
pub struct ReconSegment {
    pub offset: usize,
    pub buffer: SegmentBuffer,
}

#[derive(Clone, Debug)]
pub struct Recon(Vec<ReconSegment>);

impl Deref for Recon {
    type Target = Vec<ReconSegment>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Recon {
    pub fn new() -> Self {
        Recon(Vec::new())
    }

    pub fn add(&mut self, offset: usize, buffer: SegmentBuffer) {
        self.0.push(ReconSegment { offset, buffer })
    }

    pub fn restore(&self, buf: &mut SegmentBuffer) {
        for segment in self.0.iter() {
            buf.splice(segment.offset..segment.offset, Some(segment.buffer.clone()))
        }
    }

    pub fn split_at(&self, at: TextPosition) -> (Recon, Recon) {
        let mut rec1 = Recon::new();
        let mut rec2 = Recon::new();

        for seg in &self.0 {
            // TODO: Handle split at middle of segment
            if seg.offset < at {
                rec1.0.push(seg.clone());
            } else {
                rec2.0.push(ReconSegment {
                    offset: seg.offset - at,
                    buffer: seg.buffer.clone(),
                })
            }
        }

        (rec1, rec2)
    }
}
