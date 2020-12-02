use std::ops::RangeBounds;

use crate::{recon::Recon, segment::SegmentBuffer, ConcurrentOrder, State, TextPosition, TextSize};

use super::{insert::Insert, Operation, Split};
use anyhow::Result;

#[derive(Clone)]
pub struct Delete {
    pub position: TextPosition,
    pub what: Result<SegmentBuffer, TextSize>,
    pub recon: Recon,
}

impl Delete {
    pub fn buf(&self) -> Option<&SegmentBuffer> {
        self.what.as_ref().ok()
    }

    pub fn new(
        position: TextPosition,
        what: Result<SegmentBuffer, TextSize>,
        recon: Recon,
    ) -> Self {
        Delete {
            position,
            what,
            recon,
        }
    }
    pub fn reversible(position: TextPosition, what: SegmentBuffer, recon: Recon) -> Self {
        Self::new(position, Ok(what), recon)
    }
    fn nonreversible(position: TextPosition, what: TextSize, recon: Recon) -> Self {
        Self::new(position, Err(what), recon)
    }

    fn is_reversible(&self) -> bool {
        self.what.is_ok()
    }

    pub fn len(&self) -> TextSize {
        match &self.what {
            Ok(buf) => buf.len(),
            Err(size) => *size,
        }
    }

    fn range(&self) -> impl RangeBounds<TextPosition> {
        self.position..self.position + self.len()
    }

    pub fn apply(&self, buf: &mut SegmentBuffer) {
        buf.splice(self.range(), None)
    }

    fn split(&self, at: TextPosition) -> (Self, Self) {
        match &self.what {
            Ok(buf) => (
                Delete::reversible(self.position, buf.slice(0..at), Recon::new()),
                Delete::reversible(self.position, buf.slice(at..), Recon::new()),
            ),
            Err(len) => {
                let (rec1, rec2) = self.recon.split_at(at);

                (
                    Delete::nonreversible(self.position, at, rec1),
                    Delete::nonreversible(self.position + at, len - at, rec2),
                )
            }
        }
    }

    fn get_affected(operation: &Operation, buf: &SegmentBuffer) -> SegmentBuffer {
        match operation {
            Operation::Delete(delete) => {
                let mut recon_buf = buf.slice(delete.position..delete.position + delete.len());
                delete.recon.restore(&mut recon_buf);
                recon_buf
            }
            Operation::Split(split) => {
                let a = Delete::get_affected(&split.0, buf);
                let mut b = Delete::get_affected(&split.1, buf);
                b.splice(0..0, Some(a));
                b
            }
            _ => panic!("unknown op"),
        }
    }

    pub fn make_reversible(&self, transformed: &Operation, state: &State) -> Delete {
        match &self.what {
            Ok(buf) => Delete::reversible(self.position, buf.clone(), Recon::new()),
            Err(_) => Delete::reversible(
                self.position,
                Delete::get_affected(transformed, &state.buffer),
                Recon::new(),
            ),
        }
    }

    fn merge(&self, other: &Delete) -> Delete {
        if let Ok(buf) = &self.what {
            if let Ok(other_buf) = &other.what {
                if !other.is_reversible() {}
                let mut new_buf = buf.clone();
                new_buf.splice(new_buf.len()..new_buf.len(), Some(other_buf.clone()));

                Delete::reversible(self.position, new_buf, Recon::new())
            } else {
                panic!("cannot merge reversible operation with non-reversible");
            }
        } else {
            let new_len = self.len() + other.len();
            Delete::nonreversible(self.position, new_len, Recon::new())
        }
    }

    pub(crate) fn transform(&self, other: &Operation, cid: Option<ConcurrentOrder>) -> Operation {
        let cid = cid.unwrap();
        match other {
            Operation::NoOp => self.clone().into(),
            Operation::Delete(other) => {
                let pos1 = self.position;
                let pos2 = other.position;
                let len1 = self.len();
                let len2 = other.len();
                if pos1 + len1 <= pos2 {
                    Delete::new(pos1, self.what.clone(), self.recon.clone()).into()
                } else if pos1 >= pos2 + len2 {
                    Delete::new(pos1 - len2, self.what.clone(), self.recon.clone()).into()
                } else if pos2 <= pos1 && pos2 + len2 >= pos1 + len1 {
                    //     1XXXXX|
                    // 2-------------|
                    //
                    // This operation falls completely within the range of another,
                    // i.e. all data has already been removed. The resulting
                    // operation removes nothing.

                    let mut new_recon = self.recon.clone();
                    new_recon.add(
                        0,
                        other
                            .buf()
                            .expect("reversible other")
                            .slice(pos1 - pos2..pos1 - pos2 + len1),
                    );

                    if self.is_reversible() {
                        // TODO: Is new segment buffer correct?
                        Self::reversible(pos2, SegmentBuffer::new(smallvec::smallvec![]), new_recon)
                    } else {
                        Self::nonreversible(pos2, 0, new_recon)
                    }
                    .into()
                } else if pos2 <= pos1 && pos2 + len2 < pos1 + len1 {
                    //     1XXXX----|
                    // 2--------|
                    //
                    // The first part of this operation falls within the range of
                    // another.
                    let (_, mut result) = self.split(pos2 + len2 - pos1);
                    result.position = pos2;
                    result.recon = {
                        let mut new_recon = self.recon.clone();
                        new_recon.add(
                            0,
                            other.buf().expect("reversible other").slice(pos1 - pos2..),
                        );
                        new_recon
                    };
                    result.into()
                } else if pos2 > pos1 && pos2 + len2 >= pos1 + len1 {
                    // 1----XXXXX|
                    //     2--------|
                    //
                    // The second part of this operation falls within the range of
                    // another.
                    let (mut result, _) = self.split(pos2 - pos1);
                    result.recon = {
                        let mut new_recon = self.recon.clone();
                        new_recon.add(
                            result.len(),
                            other
                                .buf()
                                .expect("reversible other")
                                .slice(0..pos1 + len1 - pos2),
                        );
                        new_recon
                    };
                    result.into()
                } else if pos2 > pos1 && pos2 + len2 < pos1 + len1 {
                    // 1-----XXXXXX---|
                    //      2------|
                    //
                    // Another operation falls completely within the range of this
                    // operation. We remove that part.
                    let (r1, r2) = self.split(pos2 - pos1);
                    let (_, r2) = r2.split(len2);

                    let mut result = r1.merge(&r2);

                    result.recon = {
                        let mut new_recon = self.recon.clone();
                        new_recon.add(pos2 - pos1, other.buf().expect("reversible other").clone());
                        new_recon
                    };
                    result.into()
                } else {
                    unreachable!()
                }
            }
            Operation::Insert(insert) => {
                let pos1 = self.position;
                let len1 = self.len();
                let pos2 = insert.position;
                let len2 = insert.len();
                if pos1 + len1 <= pos2 {
                    self.clone().into()
                } else if pos2 <= pos1 {
                    Self::new(pos1 + len2, self.what.clone(), Recon::new()).into()
                } else if pos2 > pos1 && pos2 < pos1 + len1 {
                    let (a, mut b) = self.split(pos2 - pos1);
                    b.position += len2;
                    Split::new(a, b).into()
                } else {
                    unreachable!()
                }
            }
            Operation::Split(split) => {
                let a = self.transform(&split.0, Some(cid));
                let new_second = split.1.transform(&split.0, None);
                let b = a.transform(&new_second, Some(cid));
                b
            }
        }
    }

    pub fn mirror(&self) -> Operation {
        Insert::new(
            self.position,
            self.buf()
                .expect("delete should be reversible for mirroring")
                .clone(),
        )
        .into()
    }
}
