use std::io::{Error, Read};
use ar::Archive;

pub struct RawObjectFile {
    pub filename: String,
    pub data: Vec<u8>,
}

pub struct RawArchiveFile {
    pub filename: String,
    pub objects: Vec<RawObjectFile>,
}

pub struct Files {
    pub objects: Vec<RawObjectFile>,
    pub archives: Vec<RawArchiveFile>,
}

pub fn read_files(
    object_files: &Vec<String>,
    archive_files: &Vec<String>
) -> Files {
    let mut objects = Vec::new();
    let mut archives = Vec::new();

    for object_file in object_files {
        let result = read_object(object_file)
            .unwrap_or_else(|_| panic!("cannot read object file: {}", object_file));

        objects.push(result);
    }

    for archive_file in archive_files {
        let result = read_archive(archive_file)
            .unwrap_or_else(|_| panic!("cannot read object fle: {}", archive_file));

        archives.push(result);
    }

    Files {
        objects,
        archives,
    }
}

pub fn read_object(filename: &str) -> Result<RawObjectFile, Error> {
    let data = std::fs::read(filename)?;

    Ok(
        RawObjectFile {
            filename: filename.to_string(),
            data,
        }
    )
}

pub fn read_archive(filename: &str) -> Result<RawArchiveFile, Error> {
    let mut objects = Vec::new();

    let file = std::fs::File::open(filename)?;

    let mut archive = Archive::new(file);

    while let Some(entry_result) = archive.next_entry() {
        let mut entry = entry_result.unwrap();

        let entry_file_name = 
            std::str::from_utf8(entry.header().identifier())
            .unwrap()
            .to_string();

        if entry_file_name.ends_with(".o") {
            let mut data = Vec::new();

            entry.read_to_end(&mut data)
                .expect("cannot read an archive entry");

            objects.push(RawObjectFile {
                filename: entry_file_name,
                data,
            });
        }
    }

    Ok(
        RawArchiveFile {
            filename: filename.to_string(),
            objects,
        }
    )
}
