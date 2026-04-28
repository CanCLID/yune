use std::{ffi::c_void, ffi::CString, os::raw::c_int, ptr};

use crate::{
    free_candidate_fields, session_candidates_snapshot, Bool, RimeCandidate,
    RimeCandidateListIterator, RimeSessionId, FALSE, TRUE,
};

struct CandidateListState {
    candidates: Vec<yune_core::Candidate>,
}

/// Initializes an iterator over the current candidate list from the first item.
///
/// # Safety
///
/// `iterator` must be either null or a valid, writable pointer to a
/// `RimeCandidateListIterator`. When this function returns `TRUE`, the caller
/// must eventually pass the same iterator to `RimeCandidateListEnd`.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListBegin(
    session_id: RimeSessionId,
    iterator: *mut RimeCandidateListIterator,
) -> Bool {
    // SAFETY: forwarded preconditions are identical to
    // `RimeCandidateListFromIndex` with a zero start index.
    unsafe { RimeCandidateListFromIndex(session_id, iterator, 0) }
}

/// Initializes an iterator over the current candidate list from `index`.
///
/// # Safety
///
/// `iterator` must be either null or a valid, writable pointer to a
/// `RimeCandidateListIterator`. When this function returns `TRUE`, the caller
/// must eventually pass the same iterator to `RimeCandidateListEnd`.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListFromIndex(
    session_id: RimeSessionId,
    iterator: *mut RimeCandidateListIterator,
    index: c_int,
) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }

    let Some(candidates) = session_candidates_snapshot(session_id) else {
        return FALSE;
    };
    if candidates.is_empty() {
        return FALSE;
    }

    let state = Box::new(CandidateListState { candidates });
    // SAFETY: `iterator` is non-null and points to caller-owned writable
    // storage. The boxed state is released by `RimeCandidateListEnd`.
    unsafe {
        (*iterator).ptr = Box::into_raw(state).cast::<c_void>();
        (*iterator).index = index.saturating_sub(1);
        (*iterator).candidate = RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        };
    }
    TRUE
}

/// Advances a candidate list iterator and copies the current candidate.
///
/// # Safety
///
/// `iterator` must be either null or a valid pointer previously initialized by
/// `RimeCandidateListBegin` or `RimeCandidateListFromIndex`.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListNext(iterator: *mut RimeCandidateListIterator) -> Bool {
    if iterator.is_null() {
        return FALSE;
    }
    // SAFETY: `iterator` is non-null and points to caller-owned storage.
    let state = unsafe { (*iterator).ptr.cast::<CandidateListState>().as_ref() };
    let Some(state) = state else {
        return FALSE;
    };

    // SAFETY: `iterator` is non-null and points to caller-owned storage.
    let next_index = unsafe { (*iterator).index.saturating_add(1) };
    if next_index < 0 {
        // SAFETY: librime still advances the iterator index on failed lookup.
        unsafe {
            (*iterator).index = next_index;
        }
        return FALSE;
    }

    let candidate_index = next_index as usize;
    let Some(candidate) = state.candidates.get(candidate_index) else {
        // SAFETY: librime leaves the current candidate intact when advancing
        // past the end but still exposes the advanced iterator index.
        unsafe {
            (*iterator).index = next_index;
        }
        return FALSE;
    };
    let Ok(text) = CString::new(candidate.text.as_str()) else {
        return FALSE;
    };
    let comment = if candidate.comment.is_empty() {
        ptr::null_mut()
    } else {
        let Ok(comment) = CString::new(candidate.comment.as_str()) else {
            return FALSE;
        };
        comment.into_raw()
    };

    // SAFETY: `iterator` is non-null and points to caller-owned writable
    // storage; existing strings were allocated by this API during an earlier
    // successful `Next`, and new strings are owned until next/end.
    unsafe {
        free_candidate_fields(&mut (*iterator).candidate);
        (*iterator).index = next_index;
        (*iterator).candidate = RimeCandidate {
            text: text.into_raw(),
            comment,
            reserved: ptr::null_mut(),
        };
    }
    TRUE
}

/// Frees a candidate list iterator initialized by this API.
///
/// # Safety
///
/// `iterator` must be either null or a valid pointer. Any non-null nested
/// pointers must have been returned by candidate-list iterator APIs.
#[no_mangle]
pub unsafe extern "C" fn RimeCandidateListEnd(iterator: *mut RimeCandidateListIterator) {
    if iterator.is_null() {
        return;
    }

    // SAFETY: `iterator` is non-null and nested pointers are owned by this API
    // when populated by candidate-list iterator calls.
    unsafe {
        if !(*iterator).ptr.is_null() {
            drop(Box::from_raw((*iterator).ptr.cast::<CandidateListState>()));
        }
        free_candidate_fields(&mut (*iterator).candidate);
        (*iterator).ptr = ptr::null_mut();
        (*iterator).index = 0;
        (*iterator).candidate = RimeCandidate {
            text: ptr::null_mut(),
            comment: ptr::null_mut(),
            reserved: ptr::null_mut(),
        };
    }
}
