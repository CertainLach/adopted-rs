use crate::{segment::SegmentBuffer, ConcurrentOrder};

use super::Operation;

#[derive(Clone)]
pub struct Split(pub Operation, pub Operation);
impl Split {
    pub fn new(a: impl Into<Operation>, b: impl Into<Operation>) -> Self {
        Self(a.into(), b.into())
    }

    pub fn apply(&self, buf: &mut SegmentBuffer) {
        self.0.apply(buf);
        let transformed_second = self.0.transform(&self.0, None);
        transformed_second.apply(buf);
    }

    pub fn transform(&self, other: &Operation, cid: Option<ConcurrentOrder>) -> Operation {
        if let Some(cid) = cid {
            Self(
                self.0.transform(other, Some(cid)),
                self.1.transform(other, Some(cid)),
            )
        } else {
            Self(self.0.transform(other, None), self.1.transform(other, None))
        }
        .into()
    }

    pub fn mirror(&self) -> Operation {
        let new_second = self.1.transform(&self.0, None);
        Self(self.0.mirror(), new_second.mirror()).into()
    }
}
impl<A: Into<Operation>, B: Into<Operation>> From<(A, B)> for Split {
    fn from((a, b): (A, B)) -> Self {
        Self::new(a, b)
    }
}
