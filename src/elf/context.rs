use std::{collections::HashMap, rc::Weak, cell::RefCell};

use super::file::{ElfSymbol, ElfSymbolType, ElfObjectFile};

#[derive(Debug, Clone)]
pub enum SymbolEntry {
    Unresolved,
    Resolved(Weak<RefCell<ElfSymbol>>)
}

#[derive(Debug)]
pub struct Context {
    uid: u64,
    pub symbol_map: HashMap<String, SymbolEntry>,
    pub objects: Vec<ElfObjectFile>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            uid: 0,
            symbol_map: HashMap::new(),
            objects: Vec::new(),
        }
    }

    pub fn generate_unique_name(&mut self, prefix: &str) -> String {
        self.uid += 1;
        format!("{}{}", prefix, self.uid)
    }

    pub fn add_local_resolved_symbol(&mut self, symbol: Weak<RefCell<ElfSymbol>>) -> Result<(), ()> {
        let strong_symbol = symbol.upgrade().ok_or(())?;
        let symbol_mut = &mut (*strong_symbol).borrow_mut();
        
        let symbol_name = symbol_mut.name.as_ref()//.take()
            .expect("found local symbol without a name");

        if symbol_mut.section.is_none() {
            return Err(());
        }

        if self.symbol_map.contains_key(symbol_name) {
            panic!("symbol defined multiple times {}", symbol_name);
        }

        self.symbol_map.insert(symbol_name.to_owned(), SymbolEntry::Resolved(symbol));

        Result::Ok(())
    }

    pub fn resolve_symbol(&mut self, symbol: Weak<RefCell<ElfSymbol>>) -> Result<(), ()> {
        let strong_symbol = symbol.upgrade();

        if strong_symbol.is_none() {
            return Err(());
        }

        let strong_symbol = strong_symbol.unwrap();
        let symbol_mut = &mut (*strong_symbol).borrow_mut();
        
        let symbol_name = symbol_mut.name.as_ref()//.take()
            .expect("found global symbol without a name");

        let symbol_entry = self.symbol_map
            .entry(symbol_name.to_owned())
            .or_insert(SymbolEntry::Unresolved);

        if symbol_mut.sym_type == ElfSymbolType::Internal {
            if let SymbolEntry::Resolved(_) = symbol_entry {
                panic!("symbol defined multiple times");

                // TODO: return Err(()); // multiple definition
            } else {
                *symbol_entry = SymbolEntry::Resolved(symbol);
            }
        }

        Ok(())
    }
}
