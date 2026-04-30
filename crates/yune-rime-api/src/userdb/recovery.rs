use std::{io, path::Path};

use crate::resource_id::validate_user_dict_name;

use super::{open_store, snapshot::read_snapshot, store::UserDbStore};

pub(crate) fn recover_user_dict(dict_name: &str, snapshot: Option<&Path>) -> bool {
    let Some(valid_name) = validate_user_dict_name(dict_name) else {
        return false;
    };
    if open_store(&valid_name).is_ok_and(|store| store.validate()) {
        return true;
    }
    let Some(snapshot) = snapshot else {
        return false;
    };
    restore_validated_snapshot(snapshot).is_ok()
}

pub(crate) fn restore_validated_snapshot(snapshot: &Path) -> io::Result<()> {
    let (metadata, _records) = read_snapshot(snapshot)?;
    if validate_user_dict_name(&metadata.db_name).is_none() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid snapshot db_name",
        ));
    }
    super::sync::restore_snapshot(snapshot)
}
