use crate::app::{ReaderRunConfig, run_reader};
use std::ffi::{CStr, CString, c_char};
use std::path::PathBuf;

#[repr(C)]
pub struct FemreaderResult {
    pub json: *mut c_char,
    pub error: *mut c_char,
}

impl FemreaderResult {
    fn success(json: String) -> Self {
        Self {
            json: into_c_string_ptr(json),
            error: std::ptr::null_mut(),
        }
    }

    fn error(message: String) -> Self {
        Self {
            json: std::ptr::null_mut(),
            error: into_c_string_ptr(message),
        }
    }
}

fn into_c_string_ptr(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(text) => text.into_raw(),
        Err(_) => CString::new("FFI string contains an interior NUL byte")
            .expect("constant message cannot contain NUL")
            .into_raw(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn femreader_read_models(
    input_dir: *const c_char,
    pretty: u8,
    delete_cache: u8,
) -> FemreaderResult {
    if input_dir.is_null() {
        return FemreaderResult::error("input_dir pointer is null".to_string());
    }

    // SAFETY: input_dir nullability was checked above; caller must provide a valid
    // null-terminated C string for the duration of this call.
    let input_dir = unsafe { CStr::from_ptr(input_dir) };

    let input_dir = match input_dir.to_str() {
        Ok(value) => value,
        Err(_) => {
            return FemreaderResult::error(
                "input_dir must be a valid UTF-8 absolute path".to_string(),
            );
        }
    };

    let config = ReaderRunConfig {
        input_dir: PathBuf::from(input_dir),
        pretty: pretty != 0,
        delete_cache: delete_cache != 0,
    };

    match run_reader(&config) {
        Ok(json) => FemreaderResult::success(json),
        Err(error) => FemreaderResult::error(error),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn femreader_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }

    // SAFETY: pointer must come from CString::into_raw returned by this library.
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}
