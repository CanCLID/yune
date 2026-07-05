use std::{
    collections::HashMap,
    ffi::CStr,
    os::raw::{c_char, c_int},
    ptr,
    sync::{Mutex, OnceLock},
};

use crate::{rime_levers_get_api, Bool, RimeModule, FALSE, TRUE};

#[derive(Default)]
pub(crate) struct ModuleRegistry {
    pub(crate) modules_by_name: HashMap<String, usize>,
}

pub(crate) fn module_registry() -> &'static Mutex<ModuleRegistry> {
    static MODULE_REGISTRY: OnceLock<Mutex<ModuleRegistry>> = OnceLock::new();
    MODULE_REGISTRY.get_or_init(|| Mutex::new(ModuleRegistry::default()))
}

pub(crate) fn levers_module() -> *mut RimeModule {
    static LEVERS_MODULE: OnceLock<usize> = OnceLock::new();
    *LEVERS_MODULE.get_or_init(|| {
        Box::into_raw(Box::new(RimeModule {
            data_size: (std::mem::size_of::<RimeModule>() - std::mem::size_of::<c_int>()) as c_int,
            module_name: c"levers".as_ptr(),
            initialize: None,
            finalize: None,
            get_api: Some(rime_levers_get_api),
        })) as usize
    }) as *mut RimeModule
}

/// Registers a process-wide module pointer by its module name.
///
/// # Safety
///
/// `module` must be either null or point to a valid `RimeModule` whose
/// `module_name`, when non-null, is a valid NUL-terminated C string. The caller
/// retains ownership and must keep the module storage alive while it may be
/// returned by `RimeFindModule`.
#[no_mangle]
pub unsafe extern "C" fn RimeRegisterModule(module: *mut RimeModule) -> Bool {
    crate::ffi_guard::guard(FALSE, || {
        if module.is_null() {
            return FALSE;
        }

        // SAFETY: callers promise `module` points to a valid RimeModule.
        let module_ref = unsafe { &*module };
        if module_ref.module_name.is_null() {
            return FALSE;
        }

        // SAFETY: callers promise `module_name` is a valid NUL-terminated C string.
        let module_name = unsafe { CStr::from_ptr(module_ref.module_name) }
            .to_string_lossy()
            .into_owned();
        module_registry()
            .lock()
            .expect("module registry should not be poisoned")
            .modules_by_name
            .insert(module_name, module as usize);
        TRUE
    })
}

/// Finds a registered process-wide module by name.
///
/// # Safety
///
/// `module_name` must be either null or point to a valid NUL-terminated C
/// string.
#[no_mangle]
pub unsafe extern "C" fn RimeFindModule(module_name: *const c_char) -> *mut RimeModule {
    crate::ffi_guard::guard(std::ptr::null_mut(), || {
        if module_name.is_null() {
            return ptr::null_mut();
        }

        // SAFETY: callers promise `module_name` is a valid NUL-terminated C string.
        let module_name = unsafe { CStr::from_ptr(module_name) }.to_string_lossy();
        let registered = module_registry()
            .lock()
            .expect("module registry should not be poisoned")
            .modules_by_name
            .get(module_name.as_ref())
            .copied();
        if let Some(module) = registered {
            return module as *mut RimeModule;
        }
        if module_name == "levers" {
            return levers_module();
        }
        ptr::null_mut()
    })
}
