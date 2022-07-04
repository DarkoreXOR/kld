use std::collections::{HashMap, HashSet};

use super::UniqueId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymbolTag(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SymbolFlag {
    Global,
    Local,
    Weak,
}

#[derive(Debug)]
pub struct UnresolvedSymbol;

#[derive(Debug)]
pub struct ResolvedSymbol {
    /// Targets datatable
    pub target: UniqueId,
    pub offset: usize,
    pub flags: HashSet<SymbolFlag>,
}

impl ResolvedSymbol {
    pub fn is_global(&self) -> bool {
        self.flags.contains(&SymbolFlag::Global)
    }

    pub fn is_weak(&self) -> bool {
        self.flags.contains(&SymbolFlag::Weak)
    }
}

#[derive(Debug)]
pub enum SymbolEntry {
    Unresolved,
    Resolved(ResolvedSymbol),
}

#[derive(Debug)]
pub struct SymbolTable {
    tag_symbol_map: HashMap<SymbolTag, SymbolEntry>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            tag_symbol_map: HashMap::new(),
        }
    }

    pub fn get(&self, tag: &SymbolTag) -> Option<&SymbolEntry> {
        self.tag_symbol_map.get(&tag)
    }

    pub fn get_mut(&mut self, tag: SymbolTag) -> &mut SymbolEntry {
        self.tag_symbol_map
            .entry(tag)
            .or_insert(SymbolEntry::Unresolved)
    }
}
