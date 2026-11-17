use crate::{Patch, Seq};
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct PatchRing {
    cap: usize,
    buf: VecDeque<Patch>,
}

impl PatchRing {
    pub fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            buf: VecDeque::with_capacity(cap.max(1)),
        }
    }

    pub fn push(&mut self, patch: Patch) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(patch);
    }

    pub fn since(&self, seq: Seq) -> Option<Vec<Patch>> {
        let idx = self.buf.iter().position(|p| p.seq == seq)?;
        Some(self.buf.iter().skip(idx + 1).cloned().collect())
    }
}
