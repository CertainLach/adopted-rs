use crate::SessionId;
use std::ops::AddAssign;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct StateVector(Vec<usize>);

impl StateVector {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn add(&mut self, u: SessionId, v: usize) {
        if self.0.len() < u as usize {
            self.0.resize_with(u as usize + 1, Default::default)
        }
        self.0[u as usize] += v;
    }
    pub fn remove(&mut self, u: SessionId, v: usize) {
        self.0[u as usize] = self.0[u as usize].checked_sub(v).expect("underflow");
    }
    pub fn get(&self, u: SessionId) -> usize {
        if let Some(v) = self.0.get(u as usize) {
            *v
        } else {
            0
        }
    }
    pub fn set(&mut self, u: SessionId, value: usize) {
        if self.0.len() < u as usize {
            self.0.resize_with(u as usize + 1, Default::default)
        }
        self.0[u as usize] = value;
    }
    pub fn casually_before(&self, other: &Self) -> bool {
        self.0
            .iter()
            .enumerate()
            .all(|(u, v)| *v <= other.get(u as SessionId))
    }
    pub fn iter(&self) -> impl Iterator<Item = (SessionId, &usize)> {
        self.0.iter().enumerate().map(|(u, v)| (u as SessionId, v))
    }
    pub fn sessions(&self) -> impl Iterator<Item = SessionId> {
        (0..self.0.len()).map(|v| v as SessionId)
    }
}

impl StateVector {
    pub fn lcs(&self, other: &Self) -> Self {
        let mut out = StateVector::new();
        for (u, v) in other.0.iter().enumerate() {
            let this = self.get(u as SessionId);
            out.add(u as SessionId, v + this)
        }
        out
    }
}

impl AddAssign for StateVector {
    fn add_assign(&mut self, rhs: Self) {
        let StateVector(rhs) = rhs;
        for (u, v) in rhs.into_iter().enumerate() {
            self.add(u as SessionId, v)
        }
    }
}

impl Default for StateVector {
    fn default() -> Self {
        Self(Vec::default())
    }
}

#[cfg(test)]
pub mod tests {
    use super::StateVector;

    fn lcs() {
        let mut state = StateVector::new();
        state.add(1, 2);
        state.add(2, 3);
    }
}
