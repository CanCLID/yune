use std::{
    collections::BTreeMap,
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use super::{
    record::{UserDbMetadata, UserDbRecord, UserDbValue},
    store::UserDbStore,
};

#[derive(Clone, Debug)]
pub(crate) struct FileUserDbStore {
    path: PathBuf,
    metadata: UserDbMetadata,
    records: BTreeMap<String, UserDbValue>,
    transaction_backup: Option<(UserDbMetadata, BTreeMap<String, UserDbValue>)>,
}

impl FileUserDbStore {
    pub(crate) fn open(path: PathBuf, db_name: String, user_id: String) -> io::Result<Self> {
        if path.is_file() {
            let text = fs::read_to_string(&path)?;
            let mut store = Self::parse(path, &text)?;
            if store.metadata.user_id.is_empty() || store.metadata.user_id == "unknown" {
                store.metadata.user_id = user_id;
            }
            Ok(store)
        } else {
            Ok(Self {
                path,
                metadata: UserDbMetadata::new(db_name, user_id),
                records: BTreeMap::new(),
                transaction_backup: None,
            })
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn set_tick_to_record_max(&mut self) {
        if let Some(max_tick) = self.records.values().map(|value| value.tick).max() {
            self.metadata.tick = self.metadata.tick.max(max_tick);
        }
    }

    fn parse(path: PathBuf, text: &str) -> io::Result<Self> {
        let mut metadata = UserDbMetadata::new(String::new(), "unknown".to_owned());
        let mut records = BTreeMap::new();
        for line in text.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((first, rest)) = line.split_once('\t') else {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid userdb line",
                ));
            };
            if first.starts_with('/') {
                match first {
                    "/db_name" => metadata.db_name = rest.to_owned(),
                    "/db_type" => metadata.db_type = rest.to_owned(),
                    "/tick" => {
                        metadata.tick = rest
                            .parse()
                            .map_err(|_| io::Error::new(ErrorKind::InvalidData, "invalid tick"))?;
                    }
                    "/user_id" => metadata.user_id = rest.to_owned(),
                    "/rime_version" => metadata.rime_version = rest.to_owned(),
                    _ => {}
                }
                continue;
            }
            let Some((key, value)) = line.rsplit_once('\t') else {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "invalid userdb record",
                ));
            };
            let user_value = UserDbValue::parse(value)
                .map_err(|_| io::Error::new(ErrorKind::InvalidData, "invalid value"))?;
            let record = UserDbRecord::from_key_value(key, user_value.clone())
                .map_err(|_| io::Error::new(ErrorKind::InvalidData, "invalid key"))?;
            records.insert(record.key, record.value);
        }
        if metadata.db_name.is_empty() {
            metadata.db_name = path
                .file_name()
                .and_then(|file_name| file_name.to_str())
                .and_then(|file_name| file_name.strip_suffix(".userdb"))
                .unwrap_or("unknown")
                .to_owned();
        }
        if metadata.tick == 0 {
            metadata.tick = 1;
        }
        if metadata.db_type.is_empty() {
            metadata.db_type = "userdb".to_owned();
        }
        if metadata.db_type != "userdb" {
            return Err(io::Error::new(ErrorKind::InvalidData, "invalid metadata"));
        }
        Ok(Self {
            path,
            metadata,
            records,
            transaction_backup: None,
        })
    }

    fn serialize(&self) -> String {
        let mut output = String::from("# yune userdb\n");
        output.push_str(&format!("/db_name\t{}\n", self.metadata.db_name));
        output.push_str(&format!("/db_type\t{}\n", self.metadata.db_type));
        output.push_str(&format!("/tick\t{}\n", self.metadata.tick));
        output.push_str(&format!("/user_id\t{}\n", self.metadata.user_id));
        output.push_str(&format!("/rime_version\t{}\n", self.metadata.rime_version));
        for (key, value) in &self.records {
            output.push_str(key);
            output.push('\t');
            output.push_str(&value.pack());
            output.push('\n');
        }
        output
    }

    fn atomic_write(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = self.path.with_extension("userdb.tmp");
        fs::write(&temp, self.serialize())?;
        fs::rename(temp, &self.path)
    }
}

impl UserDbStore for FileUserDbStore {
    fn metadata(&self) -> &UserDbMetadata {
        &self.metadata
    }

    fn update_metadata(&mut self, metadata: UserDbMetadata) {
        self.metadata = metadata;
    }

    fn ordered_records(&self) -> Vec<UserDbRecord> {
        self.records
            .iter()
            .filter_map(|(key, value)| UserDbRecord::from_key_value(key, value.clone()).ok())
            .collect()
    }

    fn query_prefix(&self, prefix: &str) -> Vec<UserDbRecord> {
        self.records
            .range(prefix.to_owned()..)
            .take_while(|(key, _)| key.starts_with(prefix))
            .filter_map(|(key, value)| UserDbRecord::from_key_value(key, value.clone()).ok())
            .collect()
    }

    fn get(&self, key: &str) -> Option<UserDbValue> {
        self.records.get(key).cloned()
    }

    fn update(&mut self, record: UserDbRecord) {
        self.metadata.tick = self.metadata.tick.max(record.value.tick);
        self.records.insert(record.key, record.value);
    }

    fn delete(&mut self, key: &str) {
        self.records.remove(key);
    }

    fn begin_transaction(&mut self) -> bool {
        if self.transaction_backup.is_some() {
            return false;
        }
        self.transaction_backup = Some((self.metadata.clone(), self.records.clone()));
        true
    }

    fn commit_transaction(&mut self) -> io::Result<()> {
        if self.transaction_backup.is_none() {
            return Err(io::Error::other("no active transaction"));
        }
        if let Err(error) = self.atomic_write() {
            self.rollback();
            return Err(error);
        }
        self.transaction_backup = None;
        Ok(())
    }

    fn rollback(&mut self) -> bool {
        let Some((metadata, records)) = self.transaction_backup.take() else {
            return false;
        };
        self.metadata = metadata;
        self.records = records;
        true
    }

    fn validate(&self) -> bool {
        self.metadata.db_type == "userdb" && !self.metadata.db_name.is_empty()
    }
}
