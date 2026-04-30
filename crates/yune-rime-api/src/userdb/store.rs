use std::io;

use super::record::{UserDbMetadata, UserDbRecord, UserDbValue};

pub(crate) trait UserDbStore {
    fn metadata(&self) -> &UserDbMetadata;
    fn update_metadata(&mut self, metadata: UserDbMetadata);
    fn ordered_records(&self) -> Vec<UserDbRecord>;
    fn query_prefix(&self, prefix: &str) -> Vec<UserDbRecord>;
    fn get(&self, key: &str) -> Option<UserDbValue>;
    fn update(&mut self, record: UserDbRecord);
    fn delete(&mut self, key: &str);
    fn begin_transaction(&mut self) -> bool;
    fn commit_transaction(&mut self) -> io::Result<()>;
    fn rollback(&mut self) -> bool;
    fn validate(&self) -> bool;
}
