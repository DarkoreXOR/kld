use std::{rc::{Rc, Weak}, cell::{RefCell}, collections::HashMap};

use object::{File, Object, ObjectSymbol, ObjectSection, RelocationTarget, RelocationKind, Symbol, SectionIndex};

use crate::elf::{context::Context, utils};
use super::ParsableFile;

#[derive(Debug)]
pub enum ElfRelocationKind {
    Absolute,
    Relative,
}

#[derive(Debug)]
pub enum ElfRelocationTarget {
    Symbol(Weak<RefCell<ElfSymbol>>),
    Section(Weak<RefCell<ElfSection>>),
    Absolute,
}

#[derive(Debug)]
pub struct ElfRelocation {
    pub target: ElfRelocationTarget,
    pub size: usize,
    pub offset: usize,
    pub kind: ElfRelocationKind,
}

#[derive(Debug)]
pub struct ElfSection {
    pub file: Weak<RefCell<ElfObjectFileInner>>,
    pub name: String,
    pub data: Option<Vec<u8>>,
    /// Used for unitialized data.
    pub size: usize,
    pub alignment: usize,
    pub kind: ElfSectionKind,
    pub relocations: Vec<ElfRelocation>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ElfSymbolType {
    /// Links to implementation.
    Internal,
    /// Links to remote data.
    External,
}

#[derive(Debug)]
pub struct ElfSymbol {
    pub name: Option<String>,
    /// Symbol linked section.
    pub section: Option<Weak<RefCell<ElfSection>>>,
    pub offset: usize,
    pub stype: ElfSymbolType,
}

#[derive(Debug, Clone, Copy)]
pub enum ElfSectionKind {
    Code,
    Data,
}

#[derive(Debug)]
pub struct ElfObjectFileInner {
    pub filename: String,
    pub sections: Vec<Rc<RefCell<ElfSection>>>,
    pub symbols: Vec<Rc<RefCell<ElfSymbol>>>,
}

#[derive(Debug)]
pub struct ElfObjectFile {
    inner: Rc<RefCell<ElfObjectFileInner>>,
}

impl ElfObjectFile {
    pub fn new(filename: String) -> Self {
        let result = Self {
            inner: Rc::new(RefCell::new(ElfObjectFileInner {
                filename,
                sections: Vec::new(),
                symbols: Vec::new(),
            }))
        };

        result
    }
}

// private

impl ElfObjectFile {
    fn parse_sections_without_relocations(&self, object_file: &File) -> Vec<ElfSection> {
        let mut sections = Vec::new();

        for (index, section) in object_file.sections().enumerate() {
            sections.push(ElfSection {
                file: Rc::downgrade(&self.inner),
                name: match section.name() {
                    Ok(s) => s.to_owned(),
                    Err(_) => format!("section#{}", index),
                },
                data: match section.data() {
                    Ok(data) => Some(data.to_owned()),
                    Err(_) => None,
                },
                size: section.size() as usize,
                alignment: section.align() as usize,
                kind: if utils::is_executable_section(&section) {
                    ElfSectionKind::Code
                } else {
                    ElfSectionKind::Data
                },
                relocations: Vec::new(),
            });
        }

        sections
    }

    fn parse_sections_relocations(
        object_file: &File,
        symbols: &mut Vec<Rc<RefCell<ElfSymbol>>>,
        sections: &mut Vec<Rc<RefCell<ElfSection>>>
    ) {
        for (index, section) in object_file.sections().enumerate() {
            for (relocation_offset, relocation_data) in section.relocations() {
                let relocation = ElfRelocation {
                    target: Self::get_target_symbol(relocation_data.target(), symbols, sections),
                    size: (relocation_data.size() / 8) as usize,
                    offset: relocation_offset as usize,
                    kind: Self::get_relocation_kind(relocation_data.kind()),
                };

                let mut parsed_section = sections
                    .get_mut(index)
                    .unwrap() // always exists
                    .borrow_mut();
                
                parsed_section.relocations.push(relocation);
            }
        }
    }

    fn parse_symbols(
        context: &mut Context,
        object_file: &File,
        sections: &Vec<Rc<RefCell<ElfSection>>>
    ) -> (
        // all
        Vec<Rc<RefCell<ElfSymbol>>>,
        // local
        Vec<Rc<RefCell<ElfSymbol>>>,
        // global
        Vec<Rc<RefCell<ElfSymbol>>>
    ) {
        let mut all_symbols = Vec::new();
        let mut local_symbols = Vec::new();
        let mut global_symbols = Vec::new();

        for symbol in object_file.symbols() {
            let parent_section = Self::get_parent_section(symbol.section_index(), sections);

            // TODO replace multiple global/local methods to a single universal method

            let elf_symbol = Rc::new(RefCell::new(ElfSymbol {
                name: Self::get_symbol_special_name(&symbol, context),
                section: parent_section,
                offset: symbol.address() as usize,
                stype: if utils::is_external_symbol(&symbol) {
                    ElfSymbolType::External
                } else {
                    ElfSymbolType::Internal
                }
            }));

            if utils::is_global_symbol(&symbol) || utils::is_weak_symbol(&symbol) {
                global_symbols.push(Rc::clone(&elf_symbol));
            } else if utils::is_local_symbol(&symbol) && !utils::is_external_symbol(&symbol) {
                local_symbols.push(Rc::clone(&elf_symbol));
            }
            
            all_symbols.push(elf_symbol);
        }

        (all_symbols, local_symbols, global_symbols)
    }

    fn get_parent_section(
        section_index: Option<SectionIndex>,
        sections: &Vec<Rc<RefCell<ElfSection>>>
    ) -> Option<Weak<RefCell<ElfSection>>> {
        if let Some(index) = section_index {
            if let Some(section_cell) = sections.get(index.0) {
                return Some(Rc::downgrade(section_cell));
            }
        }

        None
    }

    fn get_target_symbol(
        target: RelocationTarget,
        symbols: &Vec<Rc<RefCell<ElfSymbol>>>,
        sections: &Vec<Rc<RefCell<ElfSection>>>
    ) -> ElfRelocationTarget {
        match target {
            RelocationTarget::Symbol(symbol_index) => {
                ElfRelocationTarget::Symbol(Rc::downgrade(symbols
                    .get(symbol_index.0)
                    .expect("cannot get target symbol")))
            }

            RelocationTarget::Section(section_index) => {
                ElfRelocationTarget::Section(Rc::downgrade(sections
                    .get(section_index.0)
                    .expect("cannot get target symbol")))
            }

            RelocationTarget::Absolute => {
                ElfRelocationTarget::Absolute
            }

            _ => unimplemented!()
        }
    }

    fn get_relocation_kind(relocation_kind: RelocationKind) -> ElfRelocationKind {
        match relocation_kind {
            RelocationKind::Absolute => ElfRelocationKind::Absolute,
            RelocationKind::Relative => ElfRelocationKind::Relative,
            //RelocationKind::Got => todo!(),
            //RelocationKind::GotRelative => todo!(),
            //RelocationKind::GotBaseRelative => todo!(),
            //RelocationKind::GotBaseOffset => todo!(),
            //RelocationKind::PltRelative => todo!(),
            //RelocationKind::ImageOffset => todo!(),
            //RelocationKind::SectionOffset => todo!(),
            //RelocationKind::SectionIndex => todo!(),
            //RelocationKind::Elf(_) => todo!(),
            //RelocationKind::MachO { value, relative } => todo!(),
            //RelocationKind::Coff(_) => todo!(),
            _ => unimplemented!()
        }
    }

    fn get_symbol_special_name(symbol: &Symbol, context: &mut Context) -> Option<String> {
        if utils::is_local_symbol(symbol) {
            let prefix = context.generate_unique_name("#unknown.local.symbol.");

            if let Some(symbol_name) = utils::get_symbol_name(symbol, None) {
                Some(format!("{}{}", symbol_name, prefix))
            } else {
                Some(prefix)
            }
        } else {
            utils::get_symbol_name(symbol, None)
        }
    }
}

impl ParsableFile for ElfObjectFile {
    fn parse(&mut self, buffer: Vec<u8>, context: &mut Context) -> Result<(), ()> {
        let object_file = object::File::parse(buffer.as_slice())
            .map_err(|_| ())?;

        let sections = self.parse_sections_without_relocations(&object_file);
        
        let mut sections = sections
            .into_iter()
            .map(|section| Rc::new(RefCell::new(section)))
            .collect();

        let (mut all_symbols, local_symbols, global_symbols) = Self::parse_symbols(
            context,
            &object_file,
            &sections
        );

        Self::parse_sections_relocations(&object_file, &mut all_symbols, &mut sections);
        
        // register sections and symbols

        let mut inner = self.inner.borrow_mut();

        inner.sections = sections;
        inner.symbols = all_symbols.clone();

        for local_symbol in local_symbols.iter() {
            drop(context.add_local_resolved_symbol(Rc::downgrade(&local_symbol)));
        }

        for global_symbol in global_symbols {
            context.resolve_symbol(Rc::downgrade(&global_symbol))?;
        }

        Result::Ok(())
    }
}
