use crate::{vector::StateVector, SessionId};

use super::Request;

#[derive(Clone)]
pub struct RedoRequest {
    pub user: SessionId,
    pub vector: StateVector,
}

impl RedoRequest {
    pub fn associated_request<'r>(
        &self,
        selfr: &Request,
        log: &'r Vec<Request>,
    ) -> Option<&'r Request> {
        let mut sequence = 1;
        let request = log.iter().rev().find(|i| {
            if i.user() != self.user {
                return false;
            }
            if i.vector().get(self.user) > self.vector.get(self.user) {
                return false;
            }
            match i {
                Request::Redo(_) => sequence += 1,
                _ => sequence -= 1,
            };

            sequence == 0
        });

        match request {
            Some(r @ Request::Undo(_)) => Some(r),
            Some(_) => unreachable!(),
            None => None,
        }
    }
}
