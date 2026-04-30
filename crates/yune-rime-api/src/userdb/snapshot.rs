use std::{fs, io, path::Path};

use crate::resource_id::validate_user_dict_name;

use super::{
    file_store::FileUserDbStore,
    record::{UserDbMetadata, UserDbRecord, UserDbValue},
    store::UserDbStore,
};

pub(crate) fn write_snapshot(store: &FileUserDbStore, snapshot: &Path) -> io::Result<()> {
    if let Some(parent) = snapshot.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut output = String::from("# Rime user dictionary\n");
    output.push_str(&format!("/db_name\t{}\n", store.metadata().db_name));
    output.push_str(&format!("/db_type\t{}\n", store.metadata().db_type));
    output.push_str(&format!("/tick\t{}\n", store.metadata().tick));
    output.push_str(&format!("/user_id\t{}\n", store.metadata().user_id));
    output.push_str(&format!(
        "/rime_version\t{}\n",
        store.metadata().rime_version
    ));
    for record in store.ordered_records() {
        output.push_str(&format!(
            "{}\t{}\t{}\n",
            record.code,
            record.phrase,
            record.value.pack()
        ));
    }
    let temp = snapshot.with_extension("userdb.txt.tmp");
    fs::write(&temp, output)?;
    fs::rename(temp, snapshot)
}

pub(crate) fn read_snapshot(snapshot: &Path) -> io::Result<(UserDbMetadata, Vec<UserDbRecord>)> {
    let text = fs::read_to_string(snapshot)?;
    let fallback_db_name = snapshot
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .and_then(|file_name| file_name.strip_suffix(".userdb.txt"));
    parse_snapshot_with_fallback(&text, fallback_db_name)
}

fn parse_snapshot_with_fallback(
    text: &str,
    fallback_db_name: Option<&str>,
) -> io::Result<(UserDbMetadata, Vec<UserDbRecord>)> {
    let mut metadata = UserDbMetadata::new(String::new(), "unknown".to_owned());
    let mut records = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let columns = line.split('\t').collect::<Vec<_>>();
        if columns.len() == 2 && columns[0].starts_with('/') {
            match columns[0] {
                "/db_name" => metadata.db_name = columns[1].to_owned(),
                "/db_type" => metadata.db_type = columns[1].to_owned(),
                "/tick" => {
                    metadata.tick = columns[1]
                        .parse()
                        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid tick"))?;
                }
                "/user_id" => metadata.user_id = columns[1].to_owned(),
                "/rime_version" => metadata.rime_version = columns[1].to_owned(),
                _ => {}
            }
            continue;
        }
        if columns.len() < 2 || columns[0].is_empty() || columns[1].is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid snapshot row",
            ));
        }
        let value = if let Some(value) = columns.get(2) {
            UserDbValue::parse(value)
        } else {
            Ok(UserDbValue::default())
        }
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid snapshot value"))?;
        let record = UserDbRecord::from_code_phrase(columns[0], columns[1], value)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid snapshot key"))?;
        records.push(record);
    }
    if metadata.db_name.is_empty() {
        metadata.db_name = fallback_db_name.unwrap_or_default().to_owned();
    }
    if metadata.tick == 0 {
        metadata.tick = 1;
    }
    if metadata.db_type.is_empty() {
        metadata.db_type = "userdb".to_owned();
    }
    if metadata.db_type != "userdb" || validate_user_dict_name(&metadata.db_name).is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid snapshot metadata",
        ));
    }
    Ok((metadata, records))
}
