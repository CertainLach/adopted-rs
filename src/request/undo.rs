use crate::{vector::StateVector, SessionId};

use super::Request;

pub struct UndoRequest {
    pub user: SessionId,
    pub vector: StateVector,
}

impl UndoRequest {
    pub fn associated_request<'r>(
        &self,
        selfr: &Request,
        log: &'r Vec<Request>,
    ) -> Option<&'r Request> {
        let mut sequence = 1;
        let request = log.iter().rev().find(|i| {
            if std::ptr::eq(*i, selfr) {
                return false;
            }
            if i.user() != self.user {
                return false;
            }
            if i.vector().get(self.user) > self.vector.get(self.user) {
                return false;
            }
            match i {
                Request::Undo(_) => sequence += 1,
                _ => sequence -= 1,
            };

            sequence == 0
        });

        match request {
            Some(r @ Request::Do(_)) => Some(r),
            Some(_) => unreachable!(),
            None => None,
        }
    }
}
