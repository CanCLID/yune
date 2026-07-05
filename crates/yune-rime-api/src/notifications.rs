use std::{
    ffi::{c_void, CString},
    sync::{Mutex, OnceLock},
};

use crate::{RimeNotificationHandler, RimeSessionId};

#[derive(Default)]
struct NotificationState {
    handler: Option<RimeNotificationHandler>,
    context_object: usize,
}

fn notification_state() -> &'static Mutex<NotificationState> {
    static NOTIFICATION_STATE: OnceLock<Mutex<NotificationState>> = OnceLock::new();
    NOTIFICATION_STATE.get_or_init(|| Mutex::new(NotificationState::default()))
}

#[no_mangle]
pub extern "C" fn RimeSetNotificationHandler(
    handler: Option<RimeNotificationHandler>,
    context_object: *mut c_void,
) {
    crate::ffi_guard::guard_void(|| {
        let mut state = notification_state()
            .lock()
            .expect("notification state should not be poisoned");
        state.handler = handler;
        state.context_object = context_object as usize;
    });
}

pub(crate) fn notify(session_id: RimeSessionId, message_type: &str, message_value: &str) {
    let (handler, context_object) = {
        let state = notification_state()
            .lock()
            .expect("notification state should not be poisoned");
        let Some(handler) = state.handler else {
            return;
        };
        (handler, state.context_object)
    };
    let Ok(message_type) = CString::new(message_type) else {
        return;
    };
    let Ok(message_value) = CString::new(message_value) else {
        return;
    };
    handler(
        context_object as *mut c_void,
        session_id,
        message_type.as_ptr(),
        message_value.as_ptr(),
    );
}
