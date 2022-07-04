use std::{collections::{HashSet, HashMap}};
use object::{Object, File, ObjectSymbol, SectionIndex, ObjectSection, Symbol, SymbolIndex, RelocationTarget, SymbolSection};
use crate::{reader::Files, core::{DataTable, SymbolTable, UniqueIdGenerator, DataEntry, DataKind, SymbolTag, UniqueId, SymbolEntry, ResolvedSymbol, SymbolFlag, RelocationEntry}};
use super::utils;

pub fn analyze(
    files: Files,
    data_table: &mut DataTable,
    symbol_table: &mut SymbolTable
) {
    let mut uid_generator = UniqueIdGenerator::new();
    let mut id_section_map = HashMap::new();

    // first pass:
    // register all symbols
    run_analyze_pass(&files, &mut uid_generator, data_table, symbol_table, &mut id_section_map, parse_firstpass);
    
    // second pass:
    // register all relocations
    run_analyze_pass(&files, &mut uid_generator, data_table, symbol_table, &mut id_section_map, parse_secondpass);
}

pub fn parse_firstpass(
    archive_filename: Option<&str>,
    object_filename: &str,
    object_file: &File,
    uid_generator: &mut UniqueIdGenerator,
    data_table: &mut DataTable,
    symbol_table: &mut SymbolTable,
    id_section_map: &mut HashMap<String, UniqueId>,
) {
    let mut used_sections_indices = HashSet::new();
    let mut section_uid_map = HashMap::new();
    let mut symbol_tag_map = HashMap::new();

    for symbol in object_file.symbols() {
        let symbol_id = utils::get_file_uid(
            archive_filename,
            object_filename,
            0,
        );

        if let Some(SectionIndex(section_index)) = symbol.section_index() {
            let section_id = utils::get_file_uid(
                archive_filename,
                object_filename,
                section_index
            );

            if !used_sections_indices.contains(&section_index) {
                let used_section = object_file
                    .section_by_index(SectionIndex(section_index))
                    .expect("cannot get section by section index");

                let uid = uid_generator.next();

                id_section_map.insert(section_id.clone(), uid);

                section_uid_map.insert(section_index, uid);

                let kind = if utils::is_executable_section(&used_section) {
                    DataKind::Code
                } else {
                    DataKind::Data
                };

                let mut data = Vec::new();

                if let Result::Ok(content) = used_section.data() {
                    data.extend(content);
                };

                data_table.register(uid, DataEntry {
                    alignment: used_section.align() as usize,
                    data,
                    size: used_section.size() as usize,
                    kind,
                    relocations: Vec::new(),
                });

                used_sections_indices.insert(section_index);
            }

            let symbol_tag = get_symbol_tag(
                &symbol_id,
                &symbol,
                object_file,
                &mut symbol_tag_map
            );

            let symbol_entry = symbol_table.get_mut(symbol_tag.clone());

            if !utils::is_external_symbol(&symbol) {
                if let SymbolEntry::Resolved(_) = *symbol_entry {
                    panic!("multiple definition of '{:?}' archive: {:?}, file: {}", symbol_tag, archive_filename, object_filename);
                }

                let uid = *section_uid_map
                    .get(&section_index)
                    .expect("cannot get uid by section index");

                let mut flags = HashSet::new();
                
                if utils::is_global_symbol(&symbol) {
                    flags.insert(SymbolFlag::Global);
                }
                else if utils::is_local_symbol(&symbol) {
                    flags.insert(SymbolFlag::Local);
                }
                else if utils::is_weak_symbol(&symbol) {
                    flags.insert(SymbolFlag::Weak);
                }

                let resolved_symbol = ResolvedSymbol {
                    target: uid,
                    offset: symbol.address() as usize,
                    flags,
                };

                *symbol_entry = SymbolEntry::Resolved(resolved_symbol);
            }
        }
    }
}

pub fn parse_secondpass(
    archive_filename: Option<&str>,
    object_filename: &str,
    object_file: &File,
    _uid_generator: &mut UniqueIdGenerator,
    data_table: &mut DataTable,
    symbol_table: &mut SymbolTable,
    id_section_map: &mut HashMap<String, UniqueId>,
) {
    let mut symbol_tag_map = HashMap::new();

    for section in object_file.sections() {
        let section_index = section.index().0;

        let section_id = utils::get_file_uid(
            archive_filename,
            object_filename,
            section_index
        );

        let uid = match id_section_map.get(&section_id) {
            Some(uid) => uid,
            _ => continue, // section is not registered, there are no relocations
        };

        let mut relocations = Vec::new();
        
        for (rel_offset, rel_data) in section.relocations() {
            let rtype = utils::get_relocation_type(&rel_data)
                .expect("cannot get relocation type");

            let relocation_symbol = match rel_data.target() {
                RelocationTarget::Symbol(symbol_index) => {
                    object_file
                        .symbol_by_index(symbol_index)
                        .expect("cannot find symbol by specified index")
                }

                _ => unimplemented!()
            };

            let symbol_id = utils::get_file_uid(
                archive_filename,
                object_filename,
                0,
            );

            // local relocations across sections

            if let SymbolSection::Section(SectionIndex(relocation_section_index)) = relocation_symbol.section() {
                let relocation_section_id = utils::get_file_uid(
                    archive_filename,
                    object_filename,
                    relocation_section_index
                );

                /*
                let symbol_name = utils::get_symbol_name(
                    &relocation_symbol, 
                    Some(object_file)
                ).expect("cannot get symbol tag");
                */

                let symbol_tag = get_symbol_tag(
                    &symbol_id,
                    &relocation_symbol,
                    object_file,
                    &mut symbol_tag_map
                );
        
                /*
                let relocation_section_uid = *id_section_map
                    .get(&relocation_section_id)
                    .expect("cannot find section id");
                */

                relocations.push(RelocationEntry {
                    target: symbol_tag,
                    size: (rel_data.size() / 8) as usize,
                    rtype,
                    offset: rel_offset as usize,
                });

                continue;
            }

            // global relocations using global name

            let is_external = utils::is_external_symbol(&relocation_symbol);
            let is_global = utils::is_global_symbol(&relocation_symbol);

            if is_external && is_global {
                let symbol_name = utils::get_symbol_name(
                    &relocation_symbol, 
                    Some(object_file)
                ).expect("cannot get symbol tag");

                // let symbol_name = relocation_symbol
                //     .name()
                //     .expect("cannot get external symbol name");

                // let symbol_entry = symbol_table
                //     .get(&SymbolTag(symbol_name.to_owned()))
                //     .expect(&format!("cannot find symbol tag by symbol name '{}'", symbol_name));

                // let (target, target_offset) = match symbol_entry {
                //     SymbolEntry::Resolved(resolved_symbol) => 
                //         (resolved_symbol.target, resolved_symbol.offset),
                //     SymbolEntry::Unresolved => panic!("got unresolved symbol '{}'", symbol_name),
                // };

                relocations.push(RelocationEntry {
                    target: SymbolTag(symbol_name),
                    size: (rel_data.size() / 8) as usize,
                    rtype,
                    offset: rel_offset as usize,
                });

                continue;
            }

            unreachable!();
        }

        let data_entry = data_table
            .get_mut(&uid)
            .expect("cannot get data by uid");

        data_entry.relocations = relocations;
    }
}

fn get_symbol_tag(
    section_id: &str,
    symbol: &Symbol,
    object_file: &File,
    symbol_tag_map: &mut HashMap<SymbolIndex, SymbolTag>
) -> SymbolTag {
    // existing
    if let Some(symbol_tag) = symbol_tag_map.get(&symbol.index()) {
        return symbol_tag.clone();
    }

    // new

    let symbol_tag = if utils::is_local_symbol(symbol) {
        let symbol_name = utils::get_symbol_name(symbol, Some(object_file))
            .expect("cannot get local symbol name");

        SymbolTag(format!("{}/{}", section_id, symbol_name))

        //SymbolTag(symbol_name)
    } else {
        let symbol_name = utils::get_symbol_name(symbol, None)
            .expect("cannot get global symbol name");

        SymbolTag(symbol_name)
    };

    symbol_tag_map.insert(symbol.index(), symbol_tag.clone());

    symbol_tag
}

fn run_analyze_pass<F>(
    files: &Files,
    uid_generator: &mut UniqueIdGenerator,
    data_table: &mut DataTable,
    symbol_table: &mut SymbolTable,
    id_section_map: &mut HashMap<String, UniqueId>,
    pass_fn: F
) where F: Fn(Option<&str>, &str, &File, &mut UniqueIdGenerator, &mut DataTable, &mut SymbolTable, &mut HashMap<String, UniqueId>) {
    for raw_object_file in files.objects.iter() {
        let object_file = object::File::parse(raw_object_file.data.as_slice())
            .expect("cannot parse object file");

        pass_fn(
            None,
            &raw_object_file.filename,
            &object_file,
            uid_generator,
            data_table,
            symbol_table,
            id_section_map,
        );
    }

    for raw_archive_file in files.archives.iter() {
        for raw_object_file in raw_archive_file.objects.iter() {
            let object_file = File::parse(raw_object_file.data.as_slice())
                .expect("cannot parse object file");

            pass_fn(
                Some(&raw_archive_file.filename),
                &raw_object_file.filename,
                &object_file,
                uid_generator,
                data_table,
                symbol_table,
                id_section_map,
            );
        }
    }
}
