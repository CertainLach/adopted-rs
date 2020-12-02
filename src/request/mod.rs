use crate::{vector::StateVector, SessionId};

use self::{dor::DoRequest, redo::RedoRequest, undo::UndoRequest};

pub mod dor;
pub mod redo;
pub mod undo;

pub enum Request {
    Do(DoRequest),
    Redo(RedoRequest),
    Undo(UndoRequest),
}

impl Request {
    pub fn user(&self) -> SessionId {
        match self {
            Request::Do(dor) => dor.user,
            Request::Redo(redo) => redo.user,
            Request::Undo(redo) => redo.user,
        }
    }
    pub fn vector(&self) -> &StateVector {
        match self {
            Request::Do(dor) => &dor.vector,
            Request::Redo(redo) => &redo.vector,
            Request::Undo(undo) => &undo.vector,
        }
    }
    pub fn associated_request<'r>(&self, log: &'r Vec<Request>) -> Option<&'r Request> {
        match self {
            Request::Do(_) => unreachable!(),
            Request::Redo(redo) => redo.associated_request(self, log),
            Request::Undo(undo) => undo.associated_request(self, log),
        }
    }
    pub fn mirror(&self, by: usize) -> Request {
        match self {
            Request::Do(dor) => dor.mirror(by),
            _ => unreachable!(),
        }
    }
    pub fn fold(&self, session: SessionId, amount: usize) -> Request {
        match self {
            Request::Do(dor) => dor.fold(session, amount),
            Request::Redo(redo) => redo.fold(session, amount),
            Request::Undo(undo) => undo.fold(session, amount),
        }
    }
}
