use std::collections::HashMap;

use super::{UniqueId, SymbolTag};

#[derive(Debug, Clone, Copy)]
pub enum RelocationType {
    Absolute,
    Relative,
    PltRelative,
}

#[derive(Debug)]
pub struct RelocationEntry {
    // Targets symbol table
    pub target: SymbolTag,
    pub size: usize,
    pub rtype: RelocationType,
    pub offset: usize,
}

#[derive(Debug)]
pub enum DataKind {
    Code,
    Data,
}

#[derive(Debug)]
pub struct DataEntry {
    pub alignment: usize,
    pub data: Vec<u8>,
    pub size: usize, // for uninitialized data (BSS)
    pub kind: DataKind,
    pub relocations: Vec<RelocationEntry>,
}

#[derive(Debug)]
pub struct DataTable {
    data_map: HashMap<UniqueId, DataEntry>,
}

impl DataTable {
    pub fn new() -> Self {
        Self {
            data_map: HashMap::new(),
        }
    }

    pub fn register(&mut self, uid: UniqueId, data_entry: DataEntry) {
        self.data_map.insert(uid, data_entry);
    }

    pub fn get(&self, uid: &UniqueId) -> Option<&DataEntry> {
        self.data_map.get(uid)
    }

    pub fn get_mut(&mut self, uid: &UniqueId) -> Option<&mut DataEntry> {
        self.data_map.get_mut(uid)
    }
}
