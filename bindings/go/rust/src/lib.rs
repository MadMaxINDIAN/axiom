//! C FFI layer for the Axiom Go cgo binding.
//!
//! All functions follow a simple convention:
//! - Return value is a heap-allocated C string (UTF-8 JSON or plain text).
//! - The caller must free the returned string with `axiom_free_string`.
//! - On error the returned string is a JSON object `{"error": "..."}`.
//! - Opaque engine handle is a raw pointer to a `Box<Registry>`.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Arc, RwLock};

use axiom_core::{
    parser,
    registry::Registry,
    schema::EvaluationRequest,
};

// ---------------------------------------------------------------------------
// Handle type
// ---------------------------------------------------------------------------

pub struct AxiomHandle {
    registry: Arc<RwLock<Registry>>,
}

// ---------------------------------------------------------------------------
// Memory helpers
// ---------------------------------------------------------------------------

fn ok_json(value: impl serde::Serialize) -> *mut c_char {
    let s = serde_json::to_string(&value).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"));
    CString::new(s).unwrap_or_default().into_raw()
}

fn err_json(msg: impl std::fmt::Display) -> *mut c_char {
    let s = format!("{{\"error\":\"{}\"}}", msg.to_string().replace('"', "\\\""));
    CString::new(s).unwrap_or_default().into_raw()
}

fn ok_null() -> *mut c_char {
    CString::new("null").unwrap().into_raw()
}

unsafe fn cstr(ptr: *const c_char) -> Result<&'static str, &'static str> {
    if ptr.is_null() { return Err("null pointer"); }
    CStr::from_ptr(ptr).to_str().map_err(|_| "invalid UTF-8")
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Create a new, empty Axiom engine. Returns an opaque handle.
/// Must be freed with `axiom_destroy`.
#[no_mangle]
pub extern "C" fn axiom_create() -> *mut AxiomHandle {
    let handle = Box::new(AxiomHandle {
        registry: Arc::new(RwLock::new(Registry::new())),
    });
    Box::into_raw(handle)
}

/// Destroy an engine handle created with `axiom_create`.
/// # Safety
/// `handle` must be a valid pointer returned by `axiom_create` and must not
/// be used after this call.
#[no_mangle]
pub unsafe extern "C" fn axiom_destroy(handle: *mut AxiomHandle) {
    if !handle.is_null() {
        drop(Box::from_raw(handle));
    }
}

/// Free a string previously returned by any axiom_* function.
/// # Safety
/// `ptr` must be a pointer returned by an axiom_* function.
#[no_mangle]
pub unsafe extern "C" fn axiom_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(CString::from_raw(ptr));
    }
}

// ---------------------------------------------------------------------------
// Rule loading
// ---------------------------------------------------------------------------

/// Load a rule from a YAML string.
/// Returns `null` on success or a JSON error object on failure.
#[no_mangle]
pub unsafe extern "C" fn axiom_load_rule_yaml(
    handle: *mut AxiomHandle,
    yaml:   *const c_char,
) -> *mut c_char {
    let h   = &*handle;
    let src = match cstr(yaml) { Ok(s) => s, Err(e) => return err_json(e) };
    let rule = match parser::parse_rule_yaml_str(src) {
        Ok(r) => r,
        Err(e) => return err_json(e),
    };
    match h.registry.write().unwrap().upsert_rule(rule) {
        Ok(())  => ok_null(),
        Err(e)  => err_json(e),
    }
}

/// Load a rule from a JSON string.
/// Returns `null` on success or a JSON error object on failure.
#[no_mangle]
pub unsafe extern "C" fn axiom_load_rule_json(
    handle: *mut AxiomHandle,
    json:   *const c_char,
) -> *mut c_char {
    let h   = &*handle;
    let src = match cstr(json) { Ok(s) => s, Err(e) => return err_json(e) };
    let rule = match parser::parse_rule_json_str(src) {
        Ok(r) => r,
        Err(e) => return err_json(e),
    };
    match h.registry.write().unwrap().upsert_rule(rule) {
        Ok(())  => ok_null(),
        Err(e)  => err_json(e),
    }
}

/// Load a rule from a file path (YAML or JSON, detected by extension).
/// Returns `null` on success or a JSON error object on failure.
#[no_mangle]
pub unsafe extern "C" fn axiom_load_rule_file(
    handle: *mut AxiomHandle,
    path:   *const c_char,
) -> *mut c_char {
    let h = &*handle;
    let p = match cstr(path) { Ok(s) => s, Err(e) => return err_json(e) };
    let bytes = match std::fs::read(p) {
        Ok(b)  => b,
        Err(e) => return err_json(format!("{p}: {e}")),
    };
    let rule = if p.ends_with(".json") {
        parser::parse_rule_json(&bytes)
    } else {
        parser::parse_rule_yaml(&bytes)
    };
    let rule = match rule { Ok(r) => r, Err(e) => return err_json(e) };
    match h.registry.write().unwrap().upsert_rule(rule) {
        Ok(())  => ok_null(),
        Err(e)  => err_json(e),
    }
}

/// Load a bundle file (YAML with `rules:` and optional `rulesets:`).
/// Returns `null` on success or a JSON error object on failure.
#[no_mangle]
pub unsafe extern "C" fn axiom_load_bundle(
    handle: *mut AxiomHandle,
    path:   *const c_char,
) -> *mut c_char {
    let h = &*handle;
    let p = match cstr(path) { Ok(s) => s, Err(e) => return err_json(e) };
    let bytes = match std::fs::read(p) {
        Ok(b)  => b,
        Err(e) => return err_json(format!("{p}: {e}")),
    };
    let (rules, rulesets) = match parser::parse_bundle_yaml(&bytes) {
        Ok(v)  => v,
        Err(e) => return err_json(e),
    };
    let mut reg = h.registry.write().unwrap();
    if let Err(e) = reg.load_rules(rules) { return err_json(e); }
    for rs in rulesets { reg.upsert_ruleset(rs); }
    ok_null()
}

// ---------------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------------

/// Evaluate a JSON-encoded EvaluationRequest.
/// Returns a JSON-encoded EvaluationResponse or a JSON error object.
#[no_mangle]
pub unsafe extern "C" fn axiom_evaluate(
    handle:       *mut AxiomHandle,
    request_json: *const c_char,
) -> *mut c_char {
    let h   = &*handle;
    let src = match cstr(request_json) { Ok(s) => s, Err(e) => return err_json(e) };
    let req: EvaluationRequest = match serde_json::from_str(src) {
        Ok(r)  => r,
        Err(e) => return err_json(e),
    };
    let reg  = h.registry.read().unwrap();
    match reg.evaluate(&req) {
        Ok(resp) => ok_json(resp),
        Err(e)   => err_json(e),
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

/// Validate ARS YAML or JSON without loading it.
/// Returns `null` if valid, or a C string containing the error message.
#[no_mangle]
pub unsafe extern "C" fn axiom_validate_rule(
    source: *const c_char,
) -> *mut c_char {
    let src = match cstr(source) { Ok(s) => s, Err(e) => return err_json(e) };
    match parser::parse_rule_yaml_str(src) {
        Ok(_)  => ok_null(),
        Err(e) => {
            let s = e.to_string();
            CString::new(s).unwrap_or_default().into_raw()
        }
    }
}
