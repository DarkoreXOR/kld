use object::{elf, Symbol, ObjectSymbol, Section, ObjectSection, SymbolFlags, SectionFlags, SymbolSection, File, Object};

pub fn is_local_symbol(symbol: &Symbol) -> bool {
    if let SymbolFlags::Elf { st_info, .. } = symbol.flags() {
        (st_info >> 4) == object::elf::STB_LOCAL
    } else {
        unimplemented!()
    }
}

pub fn is_global_symbol(symbol: &Symbol) -> bool {
    if let SymbolFlags::Elf { st_info, .. } = symbol.flags() {
        (st_info >> 4) == object::elf::STB_GLOBAL
    } else {
        unimplemented!()
    }
}

pub fn is_weak_symbol(symbol: &Symbol) -> bool {
    if let SymbolFlags::Elf { st_info, .. } = symbol.flags() {
        (st_info >> 4) == object::elf::STB_WEAK
    } else {
        unimplemented!()
    }
}

pub fn is_executable_section(section: &Section) -> bool {
    if let SectionFlags::Elf { sh_flags } = section.flags() {
        sh_flags & (elf::SHF_EXECINSTR as u64) != 0
    } else {
        unimplemented!()
    }
}

pub fn get_symbol_name<'a>(symbol: &'a Symbol, object_file: Option<&File>) -> Option<String> {
    if let Ok(symbol_name) = symbol.name() {
        if !symbol_name.is_empty() {
            return Some(symbol_name.to_owned());
        }
    }

    if let Some(object_file) = object_file {
        if let SymbolSection::Section(section_index) = symbol.section() {
            if let Result::Ok(section) = object_file.section_by_index(section_index) {
                if let Result::Ok(section_name) = section.name() {
                    if !section_name.is_empty() {
                        return Some(section_name.to_owned());
                    }
                }
            }
        }
    }

    None
}

/// External symbol is a symbol that has a name and special values:
/// 
/// * st_info.type == STT_NOTYPE
/// 
/// * shndx == UNDEF
pub fn is_external_symbol(symbol: &Symbol) -> bool {
    let has_no_type = if let SymbolFlags::Elf { st_info, .. } = symbol.flags() {
        (st_info & 0x0F) == object::elf::STT_NOTYPE
    } else {
        unimplemented!()
    };

    let has_name = get_symbol_name(symbol, None).is_some();

    has_no_type && symbol.is_undefined() && has_name
}
