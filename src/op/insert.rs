use crate::{recon::Recon, segment::SegmentBuffer, ConcurrentOrder, TextPosition, TextSize};

use super::{delete::Delete, Operation};

#[derive(Clone)]
pub struct Insert {
    pub position: TextPosition,
    buffer: SegmentBuffer,
}
impl Insert {
    pub fn new(position: TextPosition, buffer: SegmentBuffer) -> Self {
        Insert { position, buffer }
    }

    pub fn apply(&self, buf: &mut SegmentBuffer) {
        buf.splice(self.position..self.position, Some(self.buffer.clone()))
    }

    pub fn cid(&self, other: &Self) -> ConcurrentOrder {
        if self.position < other.position {
            ConcurrentOrder::Other
        } else if self.position > other.position {
            ConcurrentOrder::This
        } else {
            unreachable!()
        }
    }

    pub fn len(&self) -> TextSize {
        self.buffer.len()
    }

    pub fn transform(&self, other: &Operation, cid: Option<ConcurrentOrder>) -> Operation {
        let cid = cid.unwrap();
        match other {
            Operation::NoOp => self.clone().into(),
            Operation::Delete(delete) => {
                let pos1 = self.position;
                let pos2 = delete.position;
                let len2 = delete.len();

                let str1 = self.buffer.clone();
                if pos1 >= pos2 + len2 {
                    Insert::new(pos1 - len2, str1)
                } else if pos1 < pos2 {
                    Insert::new(pos1, str1)
                } else if pos1 >= pos2 && pos1 < pos2 + len2 {
                    Insert::new(pos2, str1)
                } else {
                    unreachable!()
                }
                .into()
            }
            Operation::Insert(other) => {
                let pos1 = self.position;
                let pos2 = other.position;

                let str1 = self.buffer.clone();

                if pos1 < pos2 || (pos1 == pos2 && cid == ConcurrentOrder::Other) {
                    Insert::new(pos1, str1)
                } else if pos1 > pos2 || (pos1 == pos2 && cid == ConcurrentOrder::This) {
                    let str2 = other.buffer.clone();
                    Insert::new(pos1 + str2.len(), str1)
                } else {
                    unreachable!()
                }
                .into()
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
        Delete::reversible(self.position, self.buffer.clone(), Recon::new()).into()
    }
}
