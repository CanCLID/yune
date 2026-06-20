use std::{ffi::CString, os::raw::c_int, ptr};

use crate::{
    apply_visible_switch_radio_defaults, bool_from, clear_commit, clear_context, clear_status,
    context_has_commit_text_preview, context_has_select_labels, context_menu_settings,
    free_rime_candidates, sessions, Bool, RimeCandidate, RimeCommit, RimeContext, RimeSessionId,
    RimeStatus, FALSE, TRUE,
};

/// Copies the unread commit text for a session into a caller-provided commit.
///
/// # Safety
///
/// `commit` must be either null or a valid, writable pointer to a `RimeCommit`.
/// When this function returns `TRUE`, the caller must release `commit.text` by
/// passing the same commit object to `RimeFreeCommit`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetCommit(session_id: RimeSessionId, commit: *mut RimeCommit) -> Bool {
    if commit.is_null() {
        return FALSE;
    }

    clear_commit(commit);

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };
    let Some(text) = session.unread_commit.take() else {
        return FALSE;
    };
    let Ok(text) = CString::new(text) else {
        return FALSE;
    };

    // SAFETY: `commit` is non-null and points to caller-owned writable storage.
    unsafe {
        (*commit).text = text.into_raw();
    }
    TRUE
}

/// Copies the current composition and first candidate page into caller storage.
///
/// # Safety
///
/// `context` must be either null or a valid, writable pointer to a
/// `RimeContext` initialized with a positive `data_size`. When this function
/// returns `TRUE`, the caller must release nested strings and candidate memory
/// by passing the same context object to `RimeFreeContext`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetContext(
    session_id: RimeSessionId,
    context: *mut RimeContext,
) -> Bool {
    if context.is_null() {
        return FALSE;
    }
    // SAFETY: `context` is non-null and points to caller-owned storage.
    if unsafe { (*context).data_size } <= 0 {
        return FALSE;
    }

    clear_context(context);

    let (snapshot, hide_candidate, chord_prompt, affix_prompt_preedit) = {
        let mut registry = sessions()
            .lock()
            .expect("session registry should not be poisoned");
        let Some(session) = registry.get_session_mut(session_id) else {
            return FALSE;
        };
        apply_visible_switch_radio_defaults(session);
        let composition_input = session.engine.context().composition.input.clone();
        (
            session.engine.snapshot(),
            session.engine.get_option("_hide_candidate"),
            session
                .chord_composer
                .as_ref()
                .and_then(|composer| composer.prompt().map(ToOwned::to_owned)),
            session
                .affix_segmentors
                .iter()
                .find_map(|segmentor| segmentor.prompt_preedit(&composition_input)),
        )
    };
    let menu_settings = context_menu_settings(&snapshot.status.schema_id);
    let select_keys = match menu_settings.select_keys.as_deref() {
        Some(select_keys) if !select_keys.as_bytes().contains(&0) => {
            match CString::new(select_keys) {
                Ok(select_keys) => Some(select_keys),
                Err(_) => return FALSE,
            }
        }
        None => None,
        Some(_) => None,
    };
    let candidate_preedit = snapshot
        .context
        .candidates
        .get(snapshot.context.highlighted)
        .and_then(|candidate| candidate.preedit.clone());
    let composition = snapshot.context.composition;
    if !composition.input.is_empty() || chord_prompt.is_some() {
        let (preedit_text, length, cursor_pos, sel_start, sel_end) =
            if let Some(chord_prompt) = chord_prompt.filter(|_| composition.input.is_empty()) {
                let length = chord_prompt.len() as c_int;
                (chord_prompt, length, 0, 0, 0)
            } else if let Some((preedit, caret)) = affix_prompt_preedit {
                let length = preedit.len() as c_int;
                (preedit, length, caret as c_int, 0, caret as c_int)
            } else if let Some(preedit) = candidate_preedit {
                let length = preedit.len() as c_int;
                (preedit, length, length, 0, length)
            } else {
                (
                    composition.preedit,
                    composition.input.len() as c_int,
                    composition.caret as c_int,
                    0,
                    composition.input.len() as c_int,
                )
            };
        let Ok(preedit) = CString::new(preedit_text) else {
            return FALSE;
        };
        let commit_text_preview = if composition.input.is_empty() {
            None
        } else if unsafe { context_has_commit_text_preview(context) } {
            let preview = snapshot
                .context
                .candidates
                .get(snapshot.context.highlighted)
                .map_or_else(
                    || composition.input.clone(),
                    |candidate| candidate.commit_text_for_input(&composition.input),
                );
            match CString::new(preview) {
                Ok(preview) => Some(preview),
                Err(_) => return FALSE,
            }
        } else {
            None
        };
        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `preedit` is converted into owned C storage for the caller.
        unsafe {
            (*context).composition.length = length;
            (*context).composition.cursor_pos = cursor_pos;
            (*context).composition.sel_start = sel_start;
            (*context).composition.sel_end = sel_end;
            (*context).composition.preedit = preedit.into_raw();
            if let Some(commit_text_preview) = commit_text_preview {
                (*context).commit_text_preview = commit_text_preview.into_raw();
            }
        }
    }

    let candidates = snapshot.context.candidates;
    if !candidates.is_empty() {
        let highlighted = snapshot.context.highlighted;
        let page_size = menu_settings.page_size;
        let page_no = highlighted / page_size;
        let page_start = page_no * page_size;
        let page_end = (page_start + page_size).min(candidates.len());
        let page_candidates = &candidates[page_start..page_end];

        if hide_candidate {
            // SAFETY: `context` is non-null and points to caller-owned writable
            // storage. librime still exposes menu metadata while hiding entries.
            unsafe {
                (*context).menu.page_size =
                    c_int::try_from(page_size).expect("menu page size should fit in c_int");
                (*context).menu.page_no = page_no as c_int;
                (*context).menu.is_last_page = bool_from(page_end == candidates.len());
                (*context).menu.highlighted_candidate_index = (highlighted - page_start)
                    .min(page_candidates.len().saturating_sub(1))
                    as c_int;
                (*context).menu.num_candidates = 0;
            }
            return TRUE;
        }

        let mut rime_candidates = Vec::with_capacity(page_candidates.len());
        for candidate in page_candidates {
            let Ok(text) = CString::new(candidate.text.as_str()) else {
                free_rime_candidates(&mut rime_candidates);
                return FALSE;
            };
            let comment = if candidate.comment.is_empty() {
                ptr::null_mut()
            } else {
                let Ok(comment) = CString::new(candidate.comment.as_str()) else {
                    free_rime_candidates(&mut rime_candidates);
                    return FALSE;
                };
                comment.into_raw()
            };
            rime_candidates.push(RimeCandidate {
                text: text.into_raw(),
                comment,
                reserved: ptr::null_mut(),
            });
        }

        let select_labels = if unsafe { context_has_select_labels(context) }
            && menu_settings.select_labels.len() >= page_size
        {
            let mut labels = Vec::with_capacity(page_size);
            for label in menu_settings.select_labels.iter().take(page_size) {
                let Ok(label) = CString::new(label.as_str()) else {
                    free_rime_candidates(&mut rime_candidates);
                    return FALSE;
                };
                labels.push(label);
            }
            let mut labels = labels
                .into_iter()
                .map(CString::into_raw)
                .collect::<Vec<_>>();
            let labels_ptr = labels.as_mut_ptr();
            std::mem::forget(labels);
            Some(labels_ptr)
        } else {
            None
        };
        let num_candidates = rime_candidates.len();
        let candidates_ptr = rime_candidates.as_mut_ptr();
        std::mem::forget(rime_candidates);

        // SAFETY: `context` is non-null and points to caller-owned writable
        // storage; `candidates_ptr` owns `num_candidates` initialized entries.
        unsafe {
            (*context).menu.page_size =
                c_int::try_from(page_size).expect("menu page size should fit in c_int");
            (*context).menu.page_no = page_no as c_int;
            (*context).menu.is_last_page = bool_from(page_end == candidates.len());
            (*context).menu.highlighted_candidate_index =
                (highlighted - page_start).min(num_candidates.saturating_sub(1)) as c_int;
            (*context).menu.num_candidates = num_candidates as c_int;
            (*context).menu.candidates = candidates_ptr;
            if let Some(select_keys) = select_keys {
                (*context).menu.select_keys = select_keys.into_raw();
            }
            if let Some(select_labels) = select_labels {
                (*context).select_labels = select_labels;
            }
        }
    }

    TRUE
}

/// Copies current session status into caller storage.
///
/// # Safety
///
/// `status` must be either null or a valid, writable pointer to a
/// `RimeStatus` initialized with a positive `data_size`. When this function
/// returns `TRUE`, the caller must release nested strings by passing the same
/// status object to `RimeFreeStatus`.
#[no_mangle]
pub unsafe extern "C" fn RimeGetStatus(session_id: RimeSessionId, status: *mut RimeStatus) -> Bool {
    if status.is_null() {
        return FALSE;
    }
    // SAFETY: `status` is non-null and points to caller-owned storage.
    if unsafe { (*status).data_size } <= 0 {
        return FALSE;
    }

    clear_status(status);

    let mut registry = sessions()
        .lock()
        .expect("session registry should not be poisoned");
    let Some(session) = registry.get_session_mut(session_id) else {
        return FALSE;
    };
    let mut snapshot = session.engine.status();
    if session
        .chord_composer
        .as_ref()
        .and_then(|composer| composer.prompt())
        .is_some()
    {
        snapshot.is_composing = true;
    }
    let Ok(schema_id) = CString::new(snapshot.schema_id) else {
        return FALSE;
    };
    let Ok(schema_name) = CString::new(snapshot.schema_name) else {
        return FALSE;
    };

    // SAFETY: `status` is non-null and points to caller-owned writable storage;
    // schema strings are converted into owned C storage for the caller.
    unsafe {
        (*status).schema_id = schema_id.into_raw();
        (*status).schema_name = schema_name.into_raw();
        (*status).is_disabled = bool_from(snapshot.is_disabled);
        (*status).is_composing = bool_from(snapshot.is_composing);
        (*status).is_ascii_mode = bool_from(snapshot.is_ascii_mode);
        (*status).is_full_shape = bool_from(snapshot.is_full_shape);
        (*status).is_simplified = bool_from(snapshot.is_simplified);
        (*status).is_traditional = bool_from(snapshot.is_traditional);
        (*status).is_ascii_punct = bool_from(snapshot.is_ascii_punct);
    }
    TRUE
}
