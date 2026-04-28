use std::{ffi::CString, ptr};

use crate::{
    clear_schema_list, deployed_schema_list_entries, free_schema_list_items, Bool, RimeSchemaList,
    RimeSchemaListItem, FALSE, TRUE,
};

/// Returns the currently available schema list.
///
/// # Safety
///
/// `schema_list` must be either null or point to writable storage. When this
/// function returns `TRUE`, the caller must release nested allocations with
/// `RimeFreeSchemaList`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetSchemaList(schema_list: *mut RimeSchemaList) -> Bool {
    if schema_list.is_null() {
        return FALSE;
    }

    clear_schema_list(schema_list);
    populate_schema_list(schema_list, deployed_schema_list_entries())
}

fn populate_schema_list(schema_list: *mut RimeSchemaList, entries: Vec<(String, String)>) -> Bool {
    if entries.is_empty() {
        return FALSE;
    }

    let mut list = Vec::with_capacity(entries.len());
    for (schema_id, name) in entries {
        let Ok(schema_id) = CString::new(schema_id) else {
            free_schema_list_items(&mut list);
            return FALSE;
        };
        let Ok(name) = CString::new(name) else {
            free_schema_list_items(&mut list);
            return FALSE;
        };
        list.push(RimeSchemaListItem {
            schema_id: schema_id.into_raw(),
            name: name.into_raw(),
            reserved: ptr::null_mut(),
        });
    }
    let size = list.len();
    let list_ptr = list.as_mut_ptr();
    std::mem::forget(list);

    // SAFETY: `schema_list` is non-null and points to caller-owned writable
    // storage; `list_ptr` owns `size` initialized schema-list items.
    unsafe {
        (*schema_list).size = size;
        (*schema_list).list = list_ptr;
    }
    TRUE
}
