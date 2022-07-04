use std::collections::{BTreeMap, HashSet, HashMap};
use object::{Object, File, ObjectSymbol, SectionIndex, ObjectSection, SymbolIndex, SymbolSection, RelocationTarget};
use crate::{symbols::{SymbolTable, Symbol, SymbolData, RelocationEntry, RelocationType, SymbolTag, SymbolKind}, reader::Files, data::{DataTable, DataEntry, DataKind}};
use super::utils;

//

pub fn analyze(
    files: Files,
    data_table: &mut DataTable,
    symbol_table: &mut SymbolTable
) {
    let mut name_section_map = BTreeMap::new();
    
    // First pass:
    // - Register sections in name_section_map
    // - Parse symbols and sections, register global symbols
    //     (already implemented symbols containing code & data).
    
    //run_analyze_pass(&files, symbol_table, &mut name_section_map, parse_firstpass);
    run_analyze_pass(&files, data_table, symbol_table, &mut name_section_map, parse_firstpass2);

    // Second pass:
    // - Parse external symbols as unresolved.

    //run_analyze_pass(&files, symbol_table, &mut name_section_map, parse_secondpass);
    
    // Third pass:
    // - Parse relocations / resolve all symbols.

    //run_analyze_pass(&files, symbol_table, &mut name_section_map, parse_thirdpass);

    //
}

fn parse_firstpass(
    archive_filename: Option<&str>,
    object_filename: &str,
    object_file: &File,
    symbol_table: &mut SymbolTable,
    name_section_map: &mut BTreeMap<String, SymbolTag>,
) {
    let mut section_symbol_map = BTreeMap::new();

    for symbol in object_file.symbols() {
        if let Some(SectionIndex(idx)) = symbol.section_index() {
            let symbols_vec = section_symbol_map
                .entry(idx)
                .or_insert(Vec::new());
            
            symbols_vec.push(symbol.index().0);
        }
    }

    for section in object_file.sections() {
        // section must have global symbol with name

        log::trace!(
            "[firstpass] section[{}] = {:?}",
            section.index().0,
            section
        );

        log::trace!("[firstpass] relocs:");

        for rel in section.relocations() {
            log::trace!(">> {:?}", rel);
        }

        let section_index = section.index().0;
        let section_id = get_section_id(
            archive_filename,
            object_filename,
            section_index
        );

        let symbol_kind = if utils::is_executable_section(&section) {
            SymbolKind::Code
        } else {
            SymbolKind::Data
        };
        
        let section_tag = symbol_table.alloc_tag();

        log::trace!("[firstpass] id: {}, tag: {}", section_id, section_tag);

        name_section_map.insert(section_id, section_tag);

        if let Some(symbol_indices) = section_symbol_map.get(&section_index) {
            let mut has_global_flag = false;
            let mut section_symbols = Vec::new();

            for symbol_index in symbol_indices {
                let symbol = object_file
                    .symbol_by_index(SymbolIndex(*symbol_index))
                    .expect("cannot find symbol with specified index");

                if utils::is_global_symbol(&symbol) {
                    has_global_flag = true;
                }

                if utils::is_weak_symbol(&symbol) {
                    has_global_flag = true;
                }

                section_symbols.push(symbol);
            }

            for section_symbol in section_symbols.iter() {
                log::trace!(
                    "[firstpass] symbol[{}] = {:?}",
                    section_symbol.index().0,
                    section_symbol
                );
            }

            let names = utils::get_names(&section_symbols);

            log::trace!(
                "[firstpass] section_index: {}, has_global_symbol: {}",
                section_index,
                has_global_flag
            );

            for name in names.iter() {
                log::trace!(
                    "[firstpass] section_index: {}, symbol_name: `{}`",
                    section_index,
                    name,
                );
            }

            if has_global_flag && names.len() > 0 {
                for name in names {
                    symbol_table
                        .add_tag_name(section_tag, name)
                        .expect("cannot add name to unregistered tag");
                }

                let symbol = Symbol::Resolved(SymbolData {
                    alignment: section.align() as usize,
                    payload: match section.data() {
                        Ok(buffer) => buffer.to_owned(),
                        Err(_) => unreachable!(),
                    },
                    size: section.size() as usize,
                    kind: symbol_kind,
                    relocations: Vec::new(), // no relocations now
                });

                symbol_table
                    .set_tag_symbol(section_tag, symbol)
                    .expect("cannot set symbol to unregistered tag");
            }
        }
    }
}

fn parse_firstpass2(
    archive_filename: Option<&str>,
    object_filename: &str,
    object_file: &File,
    data_table: &mut DataTable,
    symbol_table: &mut SymbolTable,
    name_section_map: &mut BTreeMap<String, SymbolTag>,
) {
    let mut used_section_indices = HashSet::new();
    let mut symbol_section_offsets = HashSet::new();
    let mut section_symbols_map = HashMap::new();

    for symbol in object_file.symbols() {
        if let SymbolSection::Section(SectionIndex(section_index)) = symbol.section() {
            used_section_indices.insert(section_index);
            
            symbol_section_offsets.insert((section_index, symbol.address()));

            let section_symbols_set = section_symbols_map
                .entry(section_index)
                .or_insert(HashSet::new());

            section_symbols_set.insert(symbol.index().0);
        }
    }

    for used_section_index in used_section_indices {
        let used_section = object_file
            .section_by_index(SectionIndex(used_section_index))
            .expect("cannot get section by index");
        
        let data = used_section
            .data()
            .expect("cannot get section data");
        
        let kind = if utils::is_executable_section(&used_section) {
            DataKind::Code
        } else {
            DataKind::Data
        };
        
        let data_tag = data_table.register(DataEntry {
            alignment: used_section.align() as usize,
            data: data.to_owned(),
            kind,
            relocations: Vec::new(),
        });

        let symbol_indices = section_symbols_map.get(&used_section_index)
            .expect("cannot get section indices for section");

        let symbol_tag = symbol_table.alloc_tag();

        for symbol_index in symbol_indices {
            let symbol = object_file
                .symbol_by_index(SymbolIndex(*symbol_index))
                .expect("cannot get symbol by symbol index");

            let has_global_flag = 
                utils::is_global_symbol(&symbol) || 
                utils::is_weak_symbol(&symbol);

            if let Some(symbol_name) = utils::get_symbol_name(&symbol) {
                symbol_table.add_tag_name(symbol_tag, symbol_name);
            }

            let data_offset = symbol.address() as usize;

            symbol_table.set_tag_symbol(symbol_tag, Symbol::Resolved(
                SymbolData {
                    alignment: todo!(),
                    payload: todo!(),
                    size: todo!(),
                    kind: todo!(),
                    relocations: todo!(),
                }
            ));
        }
    }
}

fn parse_secondpass(
    archive_filename: Option<&str>,
    object_filename: &str,
    object_file: &File,
    symbol_table: &mut SymbolTable,
    _name_section_map: &mut BTreeMap<String, SymbolTag>,
) {
    log::trace!(
        "[second pass] processing `{}/{}`",
        archive_filename.unwrap_or(""),
        object_filename,
    );

    for symbol in object_file.symbols() {
        if utils::is_external_symbol(&symbol) {
            let name = symbol
                .name()
                .expect("external symbol must have a name");
                
            let tag = symbol_table.get_tag_by_name(name)
                .expect(&format!("cannot found tag for specified name `{}`", name));

            log::trace!("second pass: {} = {}", tag, name);
        }
    }
}

fn parse_thirdpass(
    archive_filename: Option<&str>,
    object_filename: &str,
    object_file: &File,
    symbol_table: &mut SymbolTable,
    name_section_map: &mut BTreeMap<String, SymbolTag>,
) {
    for section in object_file.sections() {

        let section_index = section.index().0;
        let section_id = get_section_id(
            archive_filename,
            object_filename,
            section_index
        );

        let section_tag = *name_section_map
            .get(&section_id)
            .expect("cannot find section id");

        let mut relocations = Vec::new();

        for relocation in section.relocations() {
            let relocation_offset = relocation.0;
            let relocation_data = relocation.1;

            let rel_type = match relocation_data.kind() {
                object::RelocationKind::Absolute => RelocationType::Absolute,
                object::RelocationKind::Relative => RelocationType::Relative,
                // object::RelocationKind::Got => todo!(),
                // object::RelocationKind::GotRelative => todo!(),
                // object::RelocationKind::GotBaseRelative => todo!(),
                // object::RelocationKind::GotBaseOffset => todo!(),
                object::RelocationKind::PltRelative => RelocationType::PltRelative,
                // object::RelocationKind::ImageOffset => todo!(),
                // object::RelocationKind::SectionOffset => todo!(),
                // object::RelocationKind::SectionIndex => todo!(),
                // object::RelocationKind::Elf(_) => todo!(),
                // object::RelocationKind::MachO { value, relative } => todo!(),
                // object::RelocationKind::Coff(_) => todo!(),
                _ => unimplemented!(),
            };

            // relocation target: local section
            
            let relocation_symbol = match relocation_data.target() {
                RelocationTarget::Symbol(symbol_index) => {
                    object_file
                        .symbol_by_index(symbol_index)
                        .expect("cannot find symbol by specified index")
                }
                _ => unimplemented!()
            };

            if let SymbolSection::Section(SectionIndex(relocation_section_index)) = relocation_symbol.section() {

                let relocation_section_id = get_section_id(
                    archive_filename,
                    object_filename,
                    relocation_section_index
                );
        
                let relocation_section_tag = *name_section_map
                    .get(&relocation_section_id)
                    .expect("cannot find section id");

                    relocations.push(RelocationEntry {
                        tag: relocation_section_tag,
                        size: (relocation_data.size() / 8) as usize,
                        rel_type,
                        offset: relocation_offset as usize,
                    });

                continue;
            }

            // relocation target: external symbol

            let is_external_symbol = utils::is_external_symbol(&relocation_symbol);

            if is_external_symbol {
                let symbol_name = relocation_symbol
                    .name()
                    .expect("cannot get external symbol name");

                let tag = symbol_table
                    .get_tag_by_name(symbol_name)
                    .expect("cannot find tag by symbol name");

                relocations.push(RelocationEntry {
                    tag,
                    size: (relocation_data.size() / 8) as usize,
                    rel_type,
                    offset: relocation_offset as usize,
                });

                continue;
            }

            unreachable!();
        }

        //if relocations.len() > 0 {
            let symbol_kind = if utils::is_executable_section(&section) {
                SymbolKind::Code
            } else {
                SymbolKind::Data
            };

            let update_fn = |symbol: &mut Symbol| {
                match symbol {
                    Symbol::Resolved(s) => {
                        s.relocations = relocations;
                    }

                    Symbol::Unresolved(_) => {
                        *symbol = Symbol::Resolved(SymbolData {
                            alignment: section.align() as usize,
                            payload: match section.data() {
                                Ok(buffer) => buffer.to_owned(),
                                Err(_) => unreachable!(),
                            },
                            size: section.size() as usize,
                            kind: symbol_kind,
                            relocations,
                        });
                    }
                }
            };

            if !symbol_table.update_tag_symbol(section_tag, update_fn) {
                panic!("cannot update symbol with specified tag");
            }
        //}
    }
}

// utils

fn run_analyze_pass<F>(
    files: &Files,
    data_table: &mut DataTable,
    symbol_table: &mut SymbolTable,
    name_section_map: &mut BTreeMap<String, SymbolTag>,
    pass_fn: F
) where F: Fn(Option<&str>, &str, &File, &mut DataTable, &mut SymbolTable, &mut BTreeMap<String, SymbolTag>) {
    for raw_object_file in files.objects.iter() {
        let object_file = object::File::parse(&*raw_object_file.data)
            .expect("cannot parse object file");

        pass_fn(
            None,
            &raw_object_file.filename,
            &object_file,
            data_table,
            symbol_table,
            name_section_map,
        );
    }

    for raw_archive_file in files.archives.iter() {
        for raw_object_file in raw_archive_file.objects.iter() {
            let object_file = File::parse(&*raw_object_file.data)
                .expect("cannot parse object file");

            pass_fn(
                Some(&raw_archive_file.filename),
                &raw_object_file.filename,
                &object_file,
                symbol_table,
                name_section_map,
            );
        }
    }
}

fn get_section_id(
    archive_filename: Option<&str>,
    object_filename: &str,
    section_index: usize
) -> String {
    let archive_name = archive_filename
        .unwrap_or(".");

    format!(
        "{}/{}/{}",
        archive_name,
        object_filename,
        section_index
    )
}
