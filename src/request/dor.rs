use crate::{op::Operation, vector::StateVector, ConcurrentOrder, SessionId, State};

use super::Request;

#[derive(Clone)]
pub struct DoRequest {
    pub user: SessionId,
    pub vector: StateVector,
    operation: Operation,
}

impl DoRequest {
    pub fn execute(&self, state: &mut State) {
        self.operation.apply(&mut state.buffer);
        state.vector.add(self.user, 1);
    }

    pub fn transform(&self, other: &Self, cid: Option<ConcurrentOrder>) -> Self {
        let new_operation = self.operation.transform(&other.operation, cid);
        Self {
            user: self.user,
            vector: {
                let mut new_vector = self.vector.clone();
                new_vector.add(other.user, 1);
                new_vector
            },
            operation: new_operation,
        }
    }

    pub fn mirror(&self, amount: usize) -> Request {
        Request::Do(DoRequest {
            user: self.user,
            vector: {
                let mut new_vector = self.vector.clone();
                new_vector.add(self.user, amount);
                new_vector
            },
            operation: self.operation.mirror(),
        })
    }

    pub fn fold(&self, user: SessionId, amount: usize) -> Request {
        assert!(amount % 2 == 0);
        DoRequest {
            user: self.user,
            vector: {
                let mut new_vector = self.vector.clone();
                new_vector.add(user, amount);
                new_vector
            },
            operation: self.operation.clone(),
        }
    }

    pub fn make_reversible(&self, translated: &DoRequest, state: &State) -> DoRequest {
        let mut result = self.clone();
        match result.operation {
            Operation::Delete(delete) => {
                result.operation = delete.make_reversible(&translated.operation, state).into()
            }
            _ => {}
        }
        result
    }
}
