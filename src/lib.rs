use std::collections::VecDeque;

use request::Request;
use segment::SegmentBuffer;
use vector::StateVector;

pub mod op;
pub mod recon;
pub mod request;
pub mod segment;
pub mod vector;

/// One user can have multiple sessions, each session - single opened editor
pub type SessionId = u16;
/// After compaction all segments are moved to NO_OWNER sessid
/// Used as tombstones in vector.rs
pub const NO_OWNER: SessionId = 0;

pub type TextSize = usize;
pub type TextPosition = usize;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum ConcurrentOrder {
    This,
    Other,
}

pub struct State {
    pub buffer: SegmentBuffer,
    vector: StateVector,
    request_queue: VecDeque<Request>,
    log: Vec<Request>,
}

impl State {
    fn translate(&self, request: &Request, target: &StateVector) -> Request {
        match request {
            Request::Do(dor) if &dor.vector == target => return Request::Do(dor.clone()),
            Request::Undo(_) | Request::Redo(_) => {
                if let Some(assoc) = request.associated_request(&self.log) {
                    let mut mirror_at = target.clone();
                    mirror_at.set(request.user(), assoc.vector().get(request.user()));

                    if self.reachable(&mirror_at) {
                        let translated = self.translate(assoc, &mirror_at);
                        let mirror_by = target.get(request.user()) - mirror_at.get(request.user());

                        return translated.mirror(mirror_by);
                    }
                }
            }
            _ => {}
        };

        for session in self.vector.sessions() {
            if session == request.user() {
                continue;
            }
            if target.get(session) <= request.vector().get(session) {
                continue;
            }
            let mut last_request = self
                .request_by_user(session, target.get(session) - 1)
                .unwrap();

            match last_request {
                Request::Undo(_) | Request::Redo(_) => {
                    let fold_by = target.get(session)
                        - last_request
                            .associated_request(&self.log)
                            .unwrap()
                            .vector()
                            .get(session);

                    if target.get(session) > fold_by {
                        let fold_at = {
                            let mut nv = target.clone();
                            nv.remove(session, fold_by);
                            nv
                        };
                        if self.reachable(&fold_at) && request.vector().casually_before(&fold_at) {
                            let translated = self.translate(request, &fold_at);
                            return translated.fold(session, fold_by);
                        }
                    }
                }
                _ => {}
            }

            let transform_at = {
                let mut value = target.clone();
                value.remove(session, 1);
                value
            };

            if transform_at.get(session) >= 0 && self.reachable(&transform_at) {
                last_request = self
                    .request_by_user(session, transform_at.get(session))
                    .unwrap();

                let r1 = self.translate(request, &transform_at);
                let r2 = self.translate(last_request, &transform_at);
            }
        }

        todo!()
    }

    fn reachable(&self, target: &StateVector) -> bool {
        self.vector
            .iter()
            .all(|(user, _)| self.reachable_user(target, user))
    }

    fn reachable_user(&self, target: &StateVector, user: SessionId) -> bool {
        let mut n = target.get(user);
        let first_request = self.first_request_by(user);
        let first_request_number = if let Some(v) = first_request {
            v.vector().get(user)
        } else {
            self.vector.get(user)
        };

        loop {
            if n == first_request_number {
                return true;
            }
            if let Some(r) = self.request_by_user(user, n - 1) {
                match r {
                    Request::Do(dor) => {
                        let mut w = dor.vector.clone();
                        w.add(dor.user, 1);
                        return w.casually_before(target);
                    }
                    Request::Redo(_) | Request::Undo(_) => {
                        if let Some(v) = r
                            .associated_request(&self.log)
                            .map(|r| r.vector().get(user))
                        {
                            n = v;
                        }
                    }
                }
            } else {
                return false;
            }
        }
    }

    fn user_requests(&self, user: SessionId) -> impl Iterator<Item = &Request> {
        self.log.iter().filter(move |r| r.user() == user)
    }

    fn request_by_user(&self, user: SessionId, get_index: usize) -> Option<&Request> {
        self.user_requests(user)
            .find(|r| r.vector().get(user) == get_index)
    }

    fn first_request_by(&self, user: SessionId) -> Option<&Request> {
        self.user_requests(user)
            .min_by_key(|r| r.vector().get(user))
    }
}
