use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_int},
    ptr,
};

use crate::{
    Bool, LeverSchemaInfo, RimeCandidate, RimeCommit, RimeComposition, RimeContext, RimeMenu,
    RimeSchemaList, RimeSchemaListItem, RimeStatus, RimeStringSlice, RimeUserDictIterator,
    UserDictListState, FALSE, TRUE,
};

pub(crate) fn non_empty_cstring_ptr(value: &CString) -> Option<*const c_char> {
    if value.as_bytes().is_empty() {
        None
    } else {
        Some(value.as_ptr())
    }
}

pub(crate) fn cstring_from_lossless_str(value: &str) -> CString {
    CString::new(value).expect("values derived from C strings or literals cannot contain NUL bytes")
}

pub(crate) fn optional_c_string(value: *const c_char) -> Option<String> {
    if value.is_null() {
        return None;
    }

    // SAFETY: callers validate that non-null optional runtime trait strings are
    // valid NUL-terminated C strings before reaching this helper.
    Some(
        unsafe { CStr::from_ptr(value) }
            .to_string_lossy()
            .into_owned(),
    )
}

pub(crate) fn empty_string_slice() -> RimeStringSlice {
    RimeStringSlice {
        str: ptr::null(),
        length: 0,
    }
}

pub(crate) fn clear_commit(commit: *mut RimeCommit) {
    // SAFETY: callers only pass non-null pointers to this helper; fields are
    // plain pointers and assigning null mirrors librime's clear macro while
    // preserving the self-versioned struct's `data_size` field.
    unsafe {
        (*commit).text = ptr::null_mut();
    }
}

pub(crate) fn clear_context(context: *mut RimeContext) {
    // SAFETY: callers only pass non-null pointers to this helper; this mirrors
    // librime's versioned struct clear by preserving `data_size` and only
    // clearing members covered by the caller-provided version.
    unsafe {
        (*context).composition = RimeComposition {
            length: 0,
            cursor_pos: 0,
            sel_start: 0,
            sel_end: 0,
            preedit: ptr::null_mut(),
        };
        (*context).menu = RimeMenu {
            page_size: 0,
            page_no: 0,
            is_last_page: FALSE,
            highlighted_candidate_index: 0,
            num_candidates: 0,
            candidates: ptr::null_mut(),
            select_keys: ptr::null_mut(),
        };
        if context_has_commit_text_preview(context) {
            (*context).commit_text_preview = ptr::null_mut();
        }
        if context_has_select_labels(context) {
            (*context).select_labels = ptr::null_mut();
        }
    }
}

pub(crate) unsafe fn context_has_commit_text_preview(context: *const RimeContext) -> bool {
    // SAFETY: callers pass a valid `RimeContext` pointer; `addr_of!` computes a
    // field address without creating an intermediate reference.
    unsafe {
        rime_struct_has_member(
            context,
            (*context).data_size,
            ptr::addr_of!((*context).commit_text_preview),
        )
    }
}

pub(crate) unsafe fn context_has_select_labels(context: *const RimeContext) -> bool {
    // SAFETY: callers pass a valid `RimeContext` pointer; `addr_of!` computes a
    // field address without creating an intermediate reference.
    unsafe {
        rime_struct_has_member(
            context,
            (*context).data_size,
            ptr::addr_of!((*context).select_labels),
        )
    }
}

pub(crate) fn rime_struct_has_member<T, U>(
    object: *const T,
    data_size: c_int,
    member: *const U,
) -> bool {
    let Ok(data_size) = usize::try_from(data_size) else {
        return false;
    };
    let bytes_after_data_size = std::mem::size_of::<c_int>().saturating_add(data_size);
    let member_offset = (member as usize).saturating_sub(object as usize);
    bytes_after_data_size > member_offset
}

pub(crate) fn clear_status(status: *mut RimeStatus) {
    // SAFETY: callers only pass non-null pointers to this helper; this mirrors
    // librime's versioned struct clear by preserving `data_size`.
    unsafe {
        (*status).schema_id = ptr::null_mut();
        (*status).schema_name = ptr::null_mut();
        (*status).is_disabled = FALSE;
        (*status).is_composing = FALSE;
        (*status).is_ascii_mode = FALSE;
        (*status).is_full_shape = FALSE;
        (*status).is_simplified = FALSE;
        (*status).is_traditional = FALSE;
        (*status).is_ascii_punct = FALSE;
    }
}

pub(crate) fn clear_schema_list(schema_list: *mut RimeSchemaList) {
    // SAFETY: callers only pass non-null pointers to this helper; fields are
    // plain integers/pointers and assigning null mirrors librime cleanup.
    unsafe {
        (*schema_list).size = 0;
        (*schema_list).list = ptr::null_mut();
    }
}

pub(crate) unsafe fn clear_user_dict_iterator(iterator: *mut RimeUserDictIterator) {
    if iterator.is_null() {
        return;
    }
    // SAFETY: `iterator` is non-null and any non-null state pointer is owned by
    // this shim after successful iterator initialization.
    unsafe {
        if !(*iterator).ptr.is_null() {
            drop(Box::from_raw((*iterator).ptr.cast::<UserDictListState>()));
        }
        (*iterator).ptr = ptr::null_mut();
        (*iterator).i = 0;
    }
}

pub(crate) fn free_context_fields(context: *mut RimeContext) {
    // SAFETY: `context` is non-null and nested pointers are owned by this API
    // when populated by `RimeGetContext`.
    unsafe {
        if !(*context).composition.preedit.is_null() {
            drop(CString::from_raw((*context).composition.preedit));
        }
        if !(*context).menu.candidates.is_null() && (*context).menu.num_candidates > 0 {
            let num_candidates = (*context).menu.num_candidates as usize;
            let mut candidates =
                Vec::from_raw_parts((*context).menu.candidates, num_candidates, num_candidates);
            free_rime_candidates(&mut candidates);
        }
        if !(*context).menu.select_keys.is_null() {
            drop(CString::from_raw((*context).menu.select_keys));
        }
        if context_has_commit_text_preview(context) && !(*context).commit_text_preview.is_null() {
            drop(CString::from_raw((*context).commit_text_preview));
        }
        if context_has_select_labels(context) && !(*context).select_labels.is_null() {
            let page_size = (*context).menu.page_size.max(0) as usize;
            let labels = Vec::from_raw_parts((*context).select_labels, page_size, page_size);
            for label in labels {
                if !label.is_null() {
                    drop(CString::from_raw(label));
                }
            }
        }
    }
}

pub(crate) fn free_schema_list_fields(schema_list: *mut RimeSchemaList) {
    // SAFETY: `schema_list` is non-null and nested pointers are owned by this
    // API when populated by `RimeGetSchemaList`.
    unsafe {
        if (*schema_list).list.is_null() {
            return;
        }
        let size = (*schema_list).size;
        let mut list = Vec::from_raw_parts((*schema_list).list, size, size);
        free_schema_list_items(&mut list);
    }
}

pub(crate) fn free_schema_list_items(list: &mut [RimeSchemaListItem]) {
    for item in list {
        if !item.schema_id.is_null() {
            // SAFETY: schema ids are allocated by `CString::into_raw` in
            // `RimeGetSchemaList` and are released at most once here.
            unsafe { drop(CString::from_raw(item.schema_id)) };
            item.schema_id = ptr::null_mut();
        }
        if !item.name.is_null() {
            // SAFETY: names are allocated by `CString::into_raw` in
            // `RimeGetSchemaList` and are released at most once here.
            unsafe { drop(CString::from_raw(item.name)) };
            item.name = ptr::null_mut();
        }
        if !item.reserved.is_null() {
            // SAFETY: levers available-schema lists store opaque
            // `LeverSchemaInfo` boxes in `reserved`; other schema-list APIs
            // keep this field null.
            unsafe { drop(Box::from_raw(item.reserved.cast::<LeverSchemaInfo>())) };
            item.reserved = ptr::null_mut();
        }
    }
}

pub(crate) fn free_status_fields(status: *mut RimeStatus) {
    // SAFETY: `status` is non-null and nested pointers are owned by this API
    // when populated by `RimeGetStatus`.
    unsafe {
        if !(*status).schema_id.is_null() {
            drop(CString::from_raw((*status).schema_id));
        }
        if !(*status).schema_name.is_null() {
            drop(CString::from_raw((*status).schema_name));
        }
    }
}

pub(crate) fn free_rime_candidates(candidates: &mut Vec<RimeCandidate>) {
    for mut candidate in candidates.drain(..) {
        free_candidate_fields(&mut candidate);
    }
}

pub(crate) fn free_candidate_fields(candidate: &mut RimeCandidate) {
    if !candidate.text.is_null() {
        // SAFETY: candidate text pointers were returned by CString::into_raw
        // while populating a RimeContext or candidate-list iterator.
        unsafe {
            drop(CString::from_raw(candidate.text));
        }
        candidate.text = ptr::null_mut();
    }
    if !candidate.comment.is_null() {
        // SAFETY: candidate comment pointers were returned by CString::into_raw
        // while populating a RimeContext or candidate-list iterator.
        unsafe {
            drop(CString::from_raw(candidate.comment));
        }
        candidate.comment = ptr::null_mut();
    }
    candidate.reserved = ptr::null_mut();
}

pub(crate) fn copy_c_string_with_strncpy_semantics(
    value: &str,
    output: *mut c_char,
    buffer_size: usize,
) {
    if buffer_size == 0 {
        return;
    }

    let bytes = value.as_bytes();
    let copy_len = bytes.len().min(buffer_size);
    // SAFETY: callers pass writable storage of `buffer_size` bytes; `copy_len`
    // is bounded by `buffer_size`, and the zero-fill mirrors `strncpy` for
    // source strings shorter than the destination buffer.
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr().cast::<c_char>(), output, copy_len);
        if copy_len < buffer_size {
            ptr::write_bytes(output.add(copy_len), 0, buffer_size - copy_len);
        }
    }
}

/// Releases nested allocations returned in a `RimeContext`.
///
/// # Safety
///
/// `context` must be null or a valid pointer previously populated by
/// `RimeGetContext`.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeContext(context: *mut RimeContext) -> Bool {
    if context.is_null() {
        return FALSE;
    }
    // SAFETY: `context` is non-null and points to caller-owned storage.
    if unsafe { (*context).data_size } <= 0 {
        return FALSE;
    }

    free_context_fields(context);
    clear_context(context);
    TRUE
}

/// Releases nested allocations returned in a `RimeStatus`.
///
/// # Safety
///
/// `status` must be null or a valid pointer previously populated by
/// `RimeGetStatus`.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeStatus(status: *mut RimeStatus) -> Bool {
    if status.is_null() {
        return FALSE;
    }
    // SAFETY: `status` is non-null and points to caller-owned storage.
    if unsafe { (*status).data_size } <= 0 {
        return FALSE;
    }

    free_status_fields(status);
    clear_status(status);
    TRUE
}

/// Releases nested allocations returned in a `RimeCommit`.
///
/// # Safety
///
/// `commit` must be null or a valid pointer previously populated by
/// `RimeGetCommit`.
#[no_mangle]
pub unsafe extern "C" fn RimeFreeCommit(commit: *mut RimeCommit) -> Bool {
    if commit.is_null() {
        return FALSE;
    }
    // SAFETY: `commit` is non-null and `text`, when non-null, was returned by
    // `CString::into_raw` while populating the commit.
    unsafe {
        if !(*commit).text.is_null() {
            drop(CString::from_raw((*commit).text));
        }
    }
    clear_commit(commit);
    TRUE
}
