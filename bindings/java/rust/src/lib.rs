use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use jni::JNIEnv;
use jni::objects::{JClass, JObject, JString, JValue};
use jni::sys::{jlong, jstring, jboolean, JNI_TRUE, JNI_FALSE};

use axiom_core::{parser, registry::Registry, schema::EvaluationRequest};

// ---------------------------------------------------------------------------
// Registry handle — stored as a raw pointer on the Java heap via a `long`.
//
// The Java `AxiomEngine` class holds a `private long nativeHandle` field that
// stores a Box<RwLock<Registry>> cast to a raw pointer.
// ---------------------------------------------------------------------------

fn box_registry(reg: Registry) -> jlong {
    let boxed = Box::new(Arc::new(RwLock::new(reg)));
    Box::into_raw(boxed) as jlong
}

unsafe fn get_registry(handle: jlong) -> &'static Arc<RwLock<Registry>> {
    &*(handle as *const Arc<RwLock<Registry>>)
}

unsafe fn drop_registry(handle: jlong) {
    let _ = Box::from_raw(handle as *mut Arc<RwLock<Registry>>);
}

fn throw(env: &mut JNIEnv, msg: &str) {
    let _ = env.throw_new("io/axiom/AxiomException", msg);
}

fn jstring_to_string(env: &mut JNIEnv, js: JString) -> Result<String, String> {
    env.get_string(&js)
        .map(|s| s.into())
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// JNI exports — method signatures match the Java native declarations
// Class: io.axiom.AxiomEngine
// ---------------------------------------------------------------------------

/// Create a new Registry and return its handle as a long.
#[no_mangle]
pub extern "system" fn Java_io_axiom_AxiomEngine_nativeCreate(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    box_registry(Registry::new())
}

/// Free the native registry.
#[no_mangle]
pub unsafe extern "system" fn Java_io_axiom_AxiomEngine_nativeDestroy(
    _env: JNIEnv,
    _obj: JObject,
    handle: jlong,
) {
    drop_registry(handle);
}

/// Load a rule from a YAML string.
#[no_mangle]
pub extern "system" fn Java_io_axiom_AxiomEngine_nativeLoadRuleYaml(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    yaml_js: JString,
) {
    let yaml = match jstring_to_string(&mut env, yaml_js) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, &e); return; }
    };
    let rule = match parser::parse_rule_yaml_str(&yaml) {
        Ok(r) => r,
        Err(e) => { throw(&mut env, &e.to_string()); return; }
    };
    let registry = unsafe { get_registry(handle) };
    if let Err(e) = registry.write().unwrap().upsert_rule(rule) {
        throw(&mut env, &e.to_string());
    }
}

/// Load a rule from a JSON string.
#[no_mangle]
pub extern "system" fn Java_io_axiom_AxiomEngine_nativeLoadRuleJson(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    json_js: JString,
) {
    let json = match jstring_to_string(&mut env, json_js) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, &e); return; }
    };
    let rule = match parser::parse_rule_json_str(&json) {
        Ok(r) => r,
        Err(e) => { throw(&mut env, &e.to_string()); return; }
    };
    let registry = unsafe { get_registry(handle) };
    if let Err(e) = registry.write().unwrap().upsert_rule(rule) {
        throw(&mut env, &e.to_string());
    }
}

/// Load a rule from a file path.
#[no_mangle]
pub extern "system" fn Java_io_axiom_AxiomEngine_nativeLoadRuleFile(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    path_js: JString,
) {
    let path = match jstring_to_string(&mut env, path_js) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, &e); return; }
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => { throw(&mut env, &format!("{path}: {e}")); return; }
    };
    let rule = if path.ends_with(".json") {
        parser::parse_rule_json(&bytes)
    } else {
        parser::parse_rule_yaml(&bytes)
    };
    match rule {
        Ok(r) => {
            let registry = unsafe { get_registry(handle) };
            if let Err(e) = registry.write().unwrap().upsert_rule(r) {
                throw(&mut env, &e.to_string());
            }
        }
        Err(e) => throw(&mut env, &e.to_string()),
    }
}

/// Load a bundle YAML file.
#[no_mangle]
pub extern "system" fn Java_io_axiom_AxiomEngine_nativeLoadBundle(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    path_js: JString,
) {
    let path = match jstring_to_string(&mut env, path_js) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, &e); return; }
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => { throw(&mut env, &format!("{path}: {e}")); return; }
    };
    let (rules, rulesets) = match parser::parse_bundle_yaml(&bytes) {
        Ok(r) => r,
        Err(e) => { throw(&mut env, &e.to_string()); return; }
    };
    let registry = unsafe { get_registry(handle) };
    let mut reg = registry.write().unwrap();
    if let Err(e) = reg.load_rules(rules) {
        throw(&mut env, &e.to_string());
        return;
    }
    for rs in rulesets { reg.upsert_ruleset(rs); }
}

/// Evaluate: accepts a JSON-serialised EvaluationRequest, returns a JSON-serialised
/// EvaluationResponse string. Throws AxiomException on error.
#[no_mangle]
pub extern "system" fn Java_io_axiom_AxiomEngine_nativeEvaluate(
    mut env: JNIEnv,
    _obj: JObject,
    handle: jlong,
    request_js: JString,
) -> jstring {
    let request_str = match jstring_to_string(&mut env, request_js) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, &e); return std::ptr::null_mut(); }
    };
    let req: EvaluationRequest = match serde_json::from_str(&request_str) {
        Ok(r) => r,
        Err(e) => { throw(&mut env, &e.to_string()); return std::ptr::null_mut(); }
    };
    let registry = unsafe { get_registry(handle) };
    let resp = match registry.read().unwrap().evaluate(&req) {
        Ok(r) => r,
        Err(e) => { throw(&mut env, &e.to_string()); return std::ptr::null_mut(); }
    };
    let json = match serde_json::to_string(&resp) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, &e.to_string()); return std::ptr::null_mut(); }
    };
    match env.new_string(json) {
        Ok(s) => s.into_raw(),
        Err(e) => { throw(&mut env, &e.to_string()); std::ptr::null_mut() }
    }
}

/// Validate an ARS source string. Returns null jstring if valid,
/// or a jstring containing the error message.
#[no_mangle]
pub extern "system" fn Java_io_axiom_AxiomEngine_nativeValidate(
    mut env: JNIEnv,
    _class: JClass,
    source_js: JString,
    is_json: jboolean,
) -> jstring {
    let source = match jstring_to_string(&mut env, source_js) {
        Ok(s) => s,
        Err(e) => { throw(&mut env, &e); return std::ptr::null_mut(); }
    };
    let result = if is_json == JNI_TRUE {
        parser::parse_rule_json_str(&source).map(|_| ())
    } else {
        parser::parse_rule_yaml_str(&source).map(|_| ())
    };
    match result {
        Ok(()) => std::ptr::null_mut(),
        Err(e) => match env.new_string(e.to_string()) {
            Ok(s) => s.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }
}
