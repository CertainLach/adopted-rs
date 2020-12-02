mod delete;
mod insert;
mod split;
use self::{delete::Delete, insert::Insert, split::Split};
use crate::{segment::SegmentBuffer, ConcurrentOrder};

#[derive(Clone)]
pub enum Operation {
    NoOp,
    Delete(Delete),
    Insert(Insert),
    Split(Box<Split>),
}

impl Operation {
    pub fn transform(&self, other: &Operation, cid: Option<ConcurrentOrder>) -> Operation {
        match self {
            Operation::NoOp => Operation::NoOp,
            Operation::Delete(delete) => delete.transform(other, cid),
            Operation::Insert(insert) => insert.transform(other, cid),
            Operation::Split(split) => split.transform(other, cid),
        }
    }
    pub fn apply(&self, buf: &mut SegmentBuffer) {
        match self {
            Operation::NoOp => {}
            Operation::Delete(delete) => delete.apply(buf),
            Operation::Insert(insert) => insert.apply(buf),
            Operation::Split(split) => split.apply(buf),
        }
    }
    pub fn mirror(&self) -> Operation {
        match self {
            Operation::NoOp => Operation::NoOp,
            Operation::Delete(delete) => delete.mirror(),
            Operation::Insert(insert) => insert.mirror(),
            Operation::Split(split) => split.mirror(),
        }
    }
}

impl From<Delete> for Operation {
    fn from(d: Delete) -> Self {
        Operation::Delete(d)
    }
}
impl From<Insert> for Operation {
    fn from(d: Insert) -> Self {
        Operation::Insert(d)
    }
}
impl From<Split> for Operation {
    fn from(s: Split) -> Self {
        Operation::Split(Box::new(s))
    }
}
