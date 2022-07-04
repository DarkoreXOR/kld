#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniqueId(u64);

pub struct UniqueIdGenerator {
    last_id: u64,
}

impl UniqueIdGenerator {
    pub fn new() -> Self {
        Self {
            last_id: 0,
        }
    }

    pub fn next(&mut self) -> UniqueId {
        self.last_id += 1;
        UniqueId(self.last_id)
    }
}
