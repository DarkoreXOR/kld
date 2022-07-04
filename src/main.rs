mod logging;
mod core;
mod reader;
mod elf;
mod writer;
mod kos_application;

use crate::{elf::{context::{Context, SymbolEntry}, file::{ElfObjectFile, ParsableFile, ElfSectionKind}}};
use std::{collections::{VecDeque, HashMap}, path::{Path, PathBuf}};
use elf::file::{ElfRelocation, ElfRelocationKind};
use kos_application::KosApplication;

struct Options {
    library_paths: Vec<String>,
    libraries: Vec<String>,
    archives: Vec<String>,
    objects: Vec<String>,
    output: String,
}

fn read_options() -> Options {
    let mut options = Options {
        library_paths: Vec::new(),
        libraries: Vec::new(),
        archives: Vec::new(),
        objects: Vec::new(),
        output: "kos_app".to_owned(),
    };

    let mut argument_iterator = std::env::args().skip(1);

    while let Some(argument) = argument_iterator.next() {
        if argument == "-L" {
            if let Some(library_path_argument) = argument_iterator.next() {
                if std::path::Path::new(&library_path_argument).exists() {
                    log::trace!("add library path: {}", library_path_argument);
                    options.library_paths.push(library_path_argument);
                }
            }
        } else if argument.starts_with("-l") {
            let library = (&argument[2..]).trim();
            log::trace!("add library: {}", library);
            options.libraries.push(library.to_owned());
        } else if std::path::Path::new(&argument).exists() {
            log::trace!("detected file in argument: {}", argument);

            // object
            if argument.ends_with(".o") {
                log::trace!("add object file");
                options.objects.push(argument);
            }
            // archive
            else if argument.ends_with(".rlib") {
                log::trace!("add archive file");
                options.archives.push(argument);
            }

        } else {
            log::trace!("skip unsupported argument: {}", argument);
        }
    }

   options
}


enum FoundLibraryFile {
    NotFound,
    Object(String),
    Archive(String),
}

fn is_file(path: &str) -> bool {
    let path_metadata = std::fs::metadata(path);

    if path_metadata.is_err() {
        return false;
    }

    let path_metadata = path_metadata.unwrap();

    path_metadata.is_file()
}

fn check_and_get_file_path(path: PathBuf) -> Result<String, ()> {
    let path = path
        .to_str()
        .ok_or(())?;

    if !is_file(path) {
        return Err(());
    }

    Ok(path.to_owned())
}

// ! TODO: replace String to &str, make it zero copy
fn search_libraries(
    library_paths: &Vec<String>,
    libraries: &Vec<String>
) -> HashMap<String, FoundLibraryFile> {
    let mut result = HashMap::new();

    for library in libraries {
        result.insert(library.to_owned(), FoundLibraryFile::NotFound);

        for library_path in library_paths {
            let directory_path = Path::new(library_path);

            let object_file_path = directory_path
                .join(format!("lib{}.o", library));

            let archive_file_path = directory_path
                .join(format!("lib{}.rlib", library));

            if let Ok(file_path) = check_and_get_file_path(object_file_path) {
                if let Some(found_library_file) = result.get(library) {
                    match found_library_file {
                        FoundLibraryFile::Object(_) | 
                        FoundLibraryFile::Archive(_) => {
                            log::trace!("override library: `{}`, path: {}", library, &file_path);
                        }

                        _ => {}
                    }
                }

                result.insert(
                    library.to_owned(), 
                    FoundLibraryFile::Object(file_path)
                );
            }
            
            if let Ok(file_path) = check_and_get_file_path(archive_file_path) {
                if let Some(found_library_file) = result.get(library) {
                    match found_library_file {
                        FoundLibraryFile::Object(_) | 
                        FoundLibraryFile::Archive(_) => {
                            log::trace!("override library: `{}`, path: {}", library, &file_path);
                        }

                        _ => {}
                    }
                }

                result.insert(
                    library.to_owned(),
                    FoundLibraryFile::Archive(file_path)
                );
            }
        }
    }

    result
}

fn parse_libraries(options: &mut Options) {
    let found_library_files = search_libraries(
        &options.library_paths,
        &options.libraries
    );

    for (library_name, found_library_file) in found_library_files {
        match found_library_file {
            FoundLibraryFile::NotFound => {
                log::trace!("library not found: {}", library_name);
            }

            FoundLibraryFile::Object(path) => {
                log::trace!("add library: {}, object file: {}", library_name, path);
                options.objects.push(path);
            }

            FoundLibraryFile::Archive(path) => {
                log::trace!("add library: {}, archive file: {}", library_name, path);
                options.archives.push(path);
            }
        }
    }
}

fn generate_symbol_map(
    _context: &Context,
    code_tag_offset_map: &HashMap::<String, (usize, usize)>,
    code_base_addr: usize,
    data_tag_offset_map: &HashMap::<String, (usize, usize)>,
    data_base_addr: usize,
) {
    let mut string_builder = String::new();

    // code

    for (tag, (offset, size)) in code_tag_offset_map.iter() {
        let begin_address = code_base_addr + offset;
        let end_address = begin_address + size;
        //let mangled_names = symbol_table.get_tag_names(*tag);

        string_builder.push_str(&format!(
            "[c] {begin_address:08X} - {end_address:08X} ({size}):\n",
            begin_address = begin_address,
            end_address = end_address - 1,
            size = end_address - begin_address,
        ));

        for mangled_name in [tag] {
            use symbolic_common::{Language, Name, NameMangling};
            use symbolic_demangle::{Demangle, DemangleOptions};

            let name = Name::new(
                mangled_name,
                NameMangling::Mangled,
                Language::Rust
            );

            let demangled_name = Demangle::try_demangle(
                &name, 
                DemangleOptions::complete()
            );

            string_builder.push_str(
                &format!("- {} ({})\n", demangled_name, mangled_name)
            );
        }

        string_builder.push_str("\n");
    }

    // data

    for (tag, (offset, size)) in data_tag_offset_map.iter() {
        let begin_address = data_base_addr + offset;
        let end_address = begin_address + size;
        //let mangled_names = symbol_table.get_tag_names(*tag);

        string_builder.push_str(&format!(
            "[d] {begin_address:08X} - {end_address:08X} ({size}):\n",
            begin_address = begin_address,
            end_address = end_address - 1,
            size = end_address - begin_address,
        ));

        for mangled_name in [tag] {
            use symbolic_common::{Language, Name, NameMangling};
            use symbolic_demangle::{Demangle, DemangleOptions};

            let name = Name::new(
                mangled_name,
                NameMangling::Mangled,
                Language::Rust
            );

            let demangled_name = Demangle::try_demangle(
                &name, 
                DemangleOptions::complete()
            );

            string_builder.push_str(
                &format!("- {} ({})\n", demangled_name, mangled_name)
            );
        }

        string_builder.push_str("\n");
    }

    std::fs::write("map.txt", string_builder)
        .expect("cannot save the symbol map on the disk");
}


fn main() {
    logging::initialize();

    std::panic::set_hook(Box::new(|panic_info| {
        log::error!("{}", panic_info);
    }));

    let args_array_string = std::env::args()
        .skip(1)
        .map(|arg| format!("\"{}\"", arg.replace("\\", "\\\\")))
        .collect::<Vec<String>>().join(", ");

    log::trace!("\"args\": [{}]", args_array_string);

    let mut options = read_options();

    parse_libraries(&mut options);

    let files = reader::read_files(
        &options.objects, 
        &options.archives
    );

    // new:

    let mut context = Context::new();

    for raw_archive_file in files.archives {
        for raw_object_file in raw_archive_file.objects {
            let mut object_file = ElfObjectFile::new(raw_object_file.filename.to_owned());
            object_file.parse(raw_object_file.data.to_owned(), &mut context).expect("cannot parse archive file");
            context.objects.push(object_file);
        }
    }

    for raw_object_file in files.objects {
        let mut object_file = ElfObjectFile::new(raw_object_file.filename.to_owned());
        object_file.parse(raw_object_file.data.to_owned(), &mut context).expect("cannot parse object file");
        context.objects.push(object_file);
    }

    log::trace!("context: {:?}", context);

    // analyze

    let entry_point_symbol_entry = context
        .symbol_map
        .get("_start")
        .expect("entry point '_start' not found");

    // generate code

    let mut code_buffer = Vec::<u8>::new();
    let mut data_buffer = Vec::<u8>::new();

    let mut code_tag_offset_map = HashMap::<String, (usize, usize)>::new();
    let mut data_tag_offset_map = HashMap::<String, (usize, usize)>::new();

    // emit code

    let mut queue = VecDeque::<(String, SymbolEntry)>::new();
    queue.push_back(("_start".to_owned(), entry_point_symbol_entry.clone()));

    while let Some((symbol_name, symbol_entry)) = queue.pop_front() {
        if code_tag_offset_map.contains_key(&symbol_name) ||
           data_tag_offset_map.contains_key(&symbol_name) {
            continue;
        }

        let strong_symbol = match symbol_entry {
            SymbolEntry::Resolved(v) => v.upgrade(),
            SymbolEntry::Unresolved => panic!("unresolved symbol"),
        };

        let strong_symbol = strong_symbol.expect("cannot get symbol");
        let strong_symbol = (*strong_symbol).borrow();

        let symbol_section = strong_symbol.section.clone().expect("cannot get resolved symbol section");
        let symbol_section = symbol_section.upgrade().expect("got empty weak section");
        let symbol_section = (*symbol_section).borrow();
        let symbol_section_data = symbol_section.data.as_ref().expect("cannot get resolved section data");


        let (buffer, offset_map, padding_byte) = match symbol_section.kind {
            ElfSectionKind::Code => {
                (&mut code_buffer, &mut code_tag_offset_map, 0x90)
            }

            ElfSectionKind::Data => {
                (&mut data_buffer, &mut data_tag_offset_map, 0x00)
            }
        };

        // add alignment padding

        while buffer.len() % symbol_section.alignment > 0 {
            buffer.push(padding_byte);
        }

        //

        let offset = buffer.len();

        offset_map.insert(symbol_name.to_owned(), (offset, symbol_section_data.len()));
        buffer.extend(symbol_section_data);

        if symbol_section_data.len() < symbol_section.size {
            for _ in 0..(symbol_section.size - symbol_section_data.len()) {
                buffer.push(padding_byte);
            }
        }

        for relocation in symbol_section.relocations.iter() {
            let relocation_symbol = match relocation.target {
                elf::file::ElfRelocationTarget::Symbol(ref weak_symbol) => weak_symbol,
                _ => unreachable!(),
            };

            let relocation_symbol1 = relocation_symbol.upgrade().expect("cannot get strong symbol");
            let relocation_symbol2 = (*relocation_symbol1).borrow();
            let relocation_symbol_name = relocation_symbol2.name.as_ref().expect("cannot get symbol name");

            let relocation_symbol_entry = context
                .symbol_map
                .get(relocation_symbol_name)
                .expect(&format!("relocation target not found '{}'", relocation_symbol_name));

            queue.push_back((relocation_symbol_name.to_owned(), relocation_symbol_entry.clone()));

            //     let relocation_symbol_entry = context
            //         .symbol_map
            //         .get(&relocation_symbol_name)
            //         .expect(&format!("relocation target not found '{}'", relocation_symbol_name));
        
            //     let (relocation_target_name, relocation_target_entry) = match relocation_symbol_entry {
            //         SymbolEntry::Resolved(resolved_symbol) => {
            //             let resolved_symbol = resolved_symbol.;
            //             let resolved_symbol_entry = ;

            //             (resolved_symbol_name, resolved_symbol_entry)
            //         }

            //         SymbolEntry::Unresolved => panic!("unresolved symbol '{}'", relocation_symbol_name),
            // };

            //queue.push_back((relocation_target_name, relocation_target_entry));
        }
    }

    log::trace!("code_tag_offset_map: {:?}\n", code_tag_offset_map);
    log::trace!("data_tag_offset_map: {:?}\n", data_tag_offset_map);

    // create executable

    let mut kos_app = KosApplication::new(
        &options.output
    );

    let (code_offset, data_offset) = kos_app.get_offsets(
        code_buffer.len(), 
        data_buffer.len()
    );

    // patch relocations

    let code_base_addr = code_offset;
    let data_base_addr = data_offset;

    generate_symbol_map(
        &context,
        &code_tag_offset_map,
        code_base_addr,
        &data_tag_offset_map,
        data_base_addr
    );

    // code
    relocate(
        &context,
        &mut code_buffer,
        &code_tag_offset_map,
        &code_tag_offset_map,
        &data_tag_offset_map,
        code_base_addr,
        data_base_addr,
        ElfSectionKind::Code,
    );

    // data
    relocate(
        &context,
        &mut data_buffer,
        &data_tag_offset_map,
        &code_tag_offset_map,
        &data_tag_offset_map,
        code_base_addr,
        data_base_addr,
        ElfSectionKind::Data,
    );

    kos_app.build(code_buffer, data_buffer, true);

    log::trace!("### END ###");

    assert!(false);
}

fn relocate(
    context: &Context,
    buffer: &mut Vec<u8>,
    offset_map: &HashMap<String, (usize, usize)>,
    code_offset_map: &HashMap<String, (usize, usize)>,
    data_offset_map: &HashMap<String, (usize, usize)>,
    code_base_addr: usize,
    data_base_addr: usize,
    kind: ElfSectionKind
) {
    for (uid, (offset, _)) in offset_map.iter() {
        let data_entry = context
            .symbol_map
            .get(uid)
            .expect(&format!("undefined symbol with uid: {}", uid));

        let data_symbol = match data_entry {
            SymbolEntry::Resolved(resolved_symbol) => {
                resolved_symbol.upgrade()
            }

            SymbolEntry::Unresolved => panic!("unresolved symbol"),
        };

        let data_symbol = data_symbol.expect("cannot get symbol");
        let data_symbol = (*data_symbol).borrow();

        let data_symbol_section = data_symbol.section
            .as_ref()
            .expect("cannot get resolved symbol section");

        let data_symbol_section = data_symbol_section
            .upgrade()
            .expect("got empty weak section");

        let data_symbol_section = (*data_symbol_section).borrow();

        let data_symbol_section = match kind {
            ElfSectionKind::Code => {
                match data_symbol_section.kind {
                    ElfSectionKind::Code => data_symbol_section,
                    _ => unreachable!()
                }
            }
            ElfSectionKind::Data => {
                match data_symbol_section.kind {
                    ElfSectionKind::Data => data_symbol_section,
                    _ => unreachable!()
                }
            }
        };

        for relocation_entry in data_symbol_section.relocations.iter() {

            let relocation_symbol = match &relocation_entry.target {
                elf::file::ElfRelocationTarget::Symbol(weak_symbol) => weak_symbol,
                _ => unreachable!(),
            };

            let relocation_symbol = relocation_symbol
                .upgrade()
                .expect("cannot get strong symbol");

            let relocation_symbol = (*relocation_symbol).borrow();

            let relocation_symbol_name = relocation_symbol
                .name
                .as_ref()
                .expect("cannot get symbol name");

            let relocation_symbol_entry = context
                .symbol_map
                .get(relocation_symbol_name)
                .expect(&format!("relocation target not found '{}'", relocation_symbol_name));

            let resolved_symbol = match relocation_symbol_entry {
                SymbolEntry::Resolved(resolved_symbol) => resolved_symbol,
                SymbolEntry::Unresolved => panic!("got unresolved symbol"),
            };

            let resolved_symbol = resolved_symbol
                .upgrade()
                .expect("cannot get strong symbol");

            let resolved_symbol = (*resolved_symbol).borrow();

            let resolved_symbol_section = resolved_symbol.section
                .as_ref()
                .expect("cannot get resolved symbol section");

            let resolved_symbol_section = resolved_symbol_section
                .upgrade()
                .expect("got empty weak section");

            let resolved_symbol_section = (*resolved_symbol_section).borrow();

            // let relocation_data_entry = data_table
            //     .get(&resolved_symbol.target)
            //     .expect(&format!("undefined symbol with uid: {:?}", relocation_entry.target));

            let target_offset = resolved_symbol.offset;

            match resolved_symbol_section.kind {
                ElfSectionKind::Code => {
                    relocate2(
                        relocation_entry,
                        &relocation_symbol_name,
                        target_offset,
                        *offset,
                        buffer,
                        code_offset_map,
                        code_base_addr,
                    )
                }

                ElfSectionKind::Data => {
                    relocate2(
                        relocation_entry,
                        &relocation_symbol_name,
                        target_offset,
                        *offset,
                        buffer,
                        data_offset_map,
                        data_base_addr
                    )
                }
            }
        }
    }
}

fn relocate2(
    relocation_entry: &ElfRelocation,
    relocation_target: &str,
    relocation_target_offset: usize,
    offset: usize,
    buffer: &mut Vec<u8>,
    offset_map: &HashMap<String, (usize, usize)>,
    base_addr: usize,
) {
    match relocation_entry.kind {
        ElfRelocationKind::Absolute => {
            let (reloc_tag_offset, _) = offset_map
                .get(relocation_target)
                .expect("undefined relocation symbol");

            let address = base_addr + reloc_tag_offset + relocation_target_offset;

            patch_abs_reloc(
                buffer,
                offset + relocation_entry.offset as usize,
                relocation_entry.size, 
                address,
                true
            );
        }

        ElfRelocationKind::Relative => {
            let (reloc_tag_offset, _) = offset_map
                .get(relocation_target)
                .expect("undefined relocation symbol");

            let address = base_addr + reloc_tag_offset + relocation_target_offset;

            patch_rel_reloc(
                buffer,
                offset + relocation_entry.offset as usize,
                base_addr,
                relocation_entry.size,
                address
            );               
        }
    }
}

fn patch_abs_reloc(buffer: &mut [u8], offset: usize, size: usize, value: usize, add_current_value: bool) {
    log::trace!("[reloc_abs_patch] off: {}, size: {}, value: {:08X}", offset, size, value);

    match size {
        4 => {
            let current_value = if add_current_value {
                u32::from_le_bytes([
                    buffer[offset + 0],
                    buffer[offset + 1],
                    buffer[offset + 2],
                    buffer[offset + 3],
                ])
            } else {
                0
            };

            (&mut buffer[offset..(offset + size)]).copy_from_slice(
                &((value as u32).wrapping_add(current_value)).to_le_bytes()
            );
        }

        _ => unreachable!()
    }
}

fn patch_rel_reloc(buffer: &mut [u8], offset: usize, base_address: usize, size: usize, value: usize) {
    log::trace!("[reloc_rel_patch] off: {} ({:08X}), size: {}, value: {:08X}", offset, offset, size, value);
 
    let relative_value = value.wrapping_sub(
        base_address.wrapping_add(
            offset.wrapping_add(size)
        )
    );

    patch_abs_reloc(buffer, offset, size, relative_value, false);
}
