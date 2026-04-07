use regex::Regex;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::schema::Operator;

thread_local! {
    static REGEX_CACHE: RefCell<HashMap<String, Regex>> = RefCell::new(HashMap::new());
}

/// Evaluate a single leaf condition.
///
/// `field_val`  — resolved value of `field` from context
/// `cmp_val`    — the `value` in the condition (may be `None` for unary ops)
/// `field2_val` — resolved value of `field2` for cross-field ops
pub fn apply(
    op: &Operator,
    field_val: &Value,
    cmp_val: Option<&Value>,
    field2_val: Option<&Value>,
) -> bool {
    match op {
        // ── Comparison ──────────────────────────────────────────────────────
        Operator::Eq  => compare_eq(field_val, cmp_val.unwrap_or(&Value::Null)),
        Operator::Neq => !compare_eq(field_val, cmp_val.unwrap_or(&Value::Null)),
        Operator::Gt  => compare_ord(field_val, cmp_val.unwrap_or(&Value::Null)) == Some(std::cmp::Ordering::Greater),
        Operator::Gte => matches!(compare_ord(field_val, cmp_val.unwrap_or(&Value::Null)), Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)),
        Operator::Lt  => compare_ord(field_val, cmp_val.unwrap_or(&Value::Null)) == Some(std::cmp::Ordering::Less),
        Operator::Lte => matches!(compare_ord(field_val, cmp_val.unwrap_or(&Value::Null)), Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)),

        // ── String ──────────────────────────────────────────────────────────
        Operator::Contains   => op_contains(field_val, cmp_val),
        Operator::StartsWith => op_starts_with(field_val, cmp_val),
        Operator::EndsWith   => op_ends_with(field_val, cmp_val),
        Operator::Matches    => op_matches(field_val, cmp_val),
        Operator::In         => op_in(field_val, cmp_val),
        Operator::NotIn      => !op_in(field_val, cmp_val),

        // ── Numeric ─────────────────────────────────────────────────────────
        Operator::Between    => op_between(field_val, cmp_val),
        Operator::Outside    => !op_between(field_val, cmp_val),
        Operator::DivisibleBy => op_divisible_by(field_val, cmp_val),

        // ── Null / empty ────────────────────────────────────────────────────
        Operator::IsNull      => field_val.is_null(),
        Operator::IsNotNull   => !field_val.is_null(),
        Operator::IsEmpty     => is_empty(field_val),
        Operator::IsNotEmpty  => !is_empty(field_val),

        // ── Date / time ─────────────────────────────────────────────────────
        Operator::Before     => op_date_before(field_val, cmp_val),
        Operator::After      => op_date_after(field_val, cmp_val),
        Operator::WithinDays => op_within_days(field_val, cmp_val),
        Operator::IsWeekday  => op_is_weekday(field_val),
        Operator::IsWeekend  => op_is_weekend(field_val),

        // ── List ────────────────────────────────────────────────────────────
        Operator::ContainsAny => op_contains_any(field_val, cmp_val),
        Operator::ContainsAll => op_contains_all(field_val, cmp_val),
        Operator::LengthEq    => op_length(field_val, cmp_val, std::cmp::Ordering::Equal),
        Operator::LengthGt    => op_length(field_val, cmp_val, std::cmp::Ordering::Greater),
        Operator::LengthLt    => op_length(field_val, cmp_val, std::cmp::Ordering::Less),

        // ── Type check ──────────────────────────────────────────────────────
        Operator::IsType      => op_is_type(field_val, cmp_val),

        // ── Cross-field ─────────────────────────────────────────────────────
        Operator::FieldGtField => {
            let f2 = field2_val.unwrap_or(&Value::Null);
            compare_ord(field_val, f2) == Some(std::cmp::Ordering::Greater)
        }
        Operator::FieldEqField => {
            let f2 = field2_val.unwrap_or(&Value::Null);
            compare_eq(field_val, f2)
        }
    }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

fn compare_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().zip(y.as_f64()).map(|(a,b)| (a - b).abs() < f64::EPSILON).unwrap_or(false)
        }
        _ => a == b,
    }
}

fn compare_ord(a: &Value, b: &Value) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().zip(y.as_f64()).and_then(|(a, b)| a.partial_cmp(&b))
        }
        (Value::String(x), Value::String(y)) => Some(x.cmp(y)),
        // ISO 8601 date strings compared lexicographically (valid for RFC 3339)
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// String operators
// ---------------------------------------------------------------------------

fn as_str(v: &Value) -> Option<&str> { v.as_str() }

fn op_contains(field: &Value, cmp: Option<&Value>) -> bool {
    match (as_str(field), cmp.and_then(as_str)) {
        (Some(f), Some(c)) => f.contains(c),
        // field is array: contains element
        (None, Some(c)) => {
            if let Value::Array(arr) = field {
                arr.iter().any(|el| el.as_str() == Some(c))
            } else { false }
        }
        _ => false,
    }
}

fn op_starts_with(field: &Value, cmp: Option<&Value>) -> bool {
    match (as_str(field), cmp.and_then(as_str)) {
        (Some(f), Some(c)) => f.starts_with(c),
        _ => false,
    }
}

fn op_ends_with(field: &Value, cmp: Option<&Value>) -> bool {
    match (as_str(field), cmp.and_then(as_str)) {
        (Some(f), Some(c)) => f.ends_with(c),
        _ => false,
    }
}

fn op_matches(field: &Value, cmp: Option<&Value>) -> bool {
    let (Some(text), Some(pattern)) = (as_str(field), cmp.and_then(as_str)) else {
        return false;
    };
    // Use thread-local compiled regex cache.
    REGEX_CACHE.with(|cache| {
        let mut map = cache.borrow_mut();
        let re = map
            .entry(pattern.to_string())
            .or_insert_with(|| Regex::new(pattern).unwrap_or_else(|_| Regex::new("(?!)").unwrap()));
        re.is_match(text)
    })
}

fn op_in(field: &Value, cmp: Option<&Value>) -> bool {
    if let Some(Value::Array(arr)) = cmp {
        arr.contains(field)
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Numeric operators
// ---------------------------------------------------------------------------

fn as_f64(v: &Value) -> Option<f64> { v.as_f64() }

fn op_between(field: &Value, cmp: Option<&Value>) -> bool {
    // value: [low, high]  inclusive on both bounds
    if let (Some(fv), Some(Value::Array(arr))) = (as_f64(field), cmp) {
        if arr.len() == 2 {
            if let (Some(lo), Some(hi)) = (as_f64(&arr[0]), as_f64(&arr[1])) {
                return fv >= lo && fv <= hi;
            }
        }
    }
    false
}

fn op_divisible_by(field: &Value, cmp: Option<&Value>) -> bool {
    if let (Some(fv), Some(divisor)) = (as_f64(field), cmp.and_then(as_f64)) {
        if divisor == 0.0 { return false; }
        (fv % divisor).abs() < f64::EPSILON
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Null / empty
// ---------------------------------------------------------------------------

fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(a)  => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Date / time operators
// ---------------------------------------------------------------------------

use chrono::{DateTime, NaiveDate, Utc, Datelike, Weekday};

fn parse_date(v: &Value) -> Option<DateTime<Utc>> {
    v.as_str().and_then(|s| s.parse::<DateTime<Utc>>().ok())
}

fn parse_naive_date(v: &Value) -> Option<NaiveDate> {
    v.as_str().and_then(|s| {
        s.parse::<DateTime<Utc>>().ok().map(|dt| dt.date_naive())
            .or_else(|| s.parse::<NaiveDate>().ok())
    })
}

fn op_date_before(field: &Value, cmp: Option<&Value>) -> bool {
    match (parse_date(field), cmp.and_then(parse_date)) {
        (Some(a), Some(b)) => a < b,
        _ => false,
    }
}

fn op_date_after(field: &Value, cmp: Option<&Value>) -> bool {
    match (parse_date(field), cmp.and_then(parse_date)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

fn op_within_days(field: &Value, cmp: Option<&Value>) -> bool {
    if let (Some(dt), Some(days)) = (parse_date(field), cmp.and_then(as_f64)) {
        let diff = (Utc::now() - dt).num_days().unsigned_abs();
        (diff as f64) < days
    } else {
        false
    }
}

fn op_is_weekday(field: &Value) -> bool {
    parse_naive_date(field)
        .map(|d| !matches!(d.weekday(), Weekday::Sat | Weekday::Sun))
        .unwrap_or(false)
}

fn op_is_weekend(field: &Value) -> bool {
    parse_naive_date(field)
        .map(|d| matches!(d.weekday(), Weekday::Sat | Weekday::Sun))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// List operators
// ---------------------------------------------------------------------------

fn op_contains_any(field: &Value, cmp: Option<&Value>) -> bool {
    if let (Value::Array(haystack), Some(Value::Array(needles))) = (field, cmp) {
        needles.iter().any(|n| haystack.contains(n))
    } else {
        false
    }
}

fn op_contains_all(field: &Value, cmp: Option<&Value>) -> bool {
    if let (Value::Array(haystack), Some(Value::Array(needles))) = (field, cmp) {
        needles.iter().all(|n| haystack.contains(n))
    } else {
        false
    }
}

fn op_length(field: &Value, cmp: Option<&Value>, expected_ord: std::cmp::Ordering) -> bool {
    let len = match field {
        Value::Array(a)  => Some(a.len()),
        Value::String(s) => Some(s.len()),
        Value::Object(o) => Some(o.len()),
        _ => None,
    };
    if let (Some(l), Some(n)) = (len, cmp.and_then(as_f64)) {
        (l as f64).partial_cmp(&n) == Some(expected_ord)
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Type check
// ---------------------------------------------------------------------------

fn op_is_type(field: &Value, cmp: Option<&Value>) -> bool {
    let type_name = cmp.and_then(as_str).unwrap_or("");
    match (type_name, field) {
        ("string",  Value::String(_))  => true,
        ("number",  Value::Number(_))  => true,
        ("boolean", Value::Bool(_))    => true,
        ("array",   Value::Array(_))   => true,
        ("object",  Value::Object(_))  => true,
        ("null",    Value::Null)       => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn eq_numbers() {
        assert!(apply(&Operator::Eq, &json!(42), Some(&json!(42)), None));
        assert!(!apply(&Operator::Eq, &json!(42), Some(&json!(43)), None));
    }

    #[test]
    fn gte() {
        assert!(apply(&Operator::Gte, &json!(10), Some(&json!(10)), None));
        assert!(apply(&Operator::Gte, &json!(11), Some(&json!(10)), None));
        assert!(!apply(&Operator::Gte, &json!(9), Some(&json!(10)), None));
    }

    #[test]
    fn between_inclusive() {
        assert!(apply(&Operator::Between, &json!(5), Some(&json!([1, 10])), None));
        assert!(apply(&Operator::Between, &json!(1), Some(&json!([1, 10])), None));
        assert!(apply(&Operator::Between, &json!(10), Some(&json!([1, 10])), None));
        assert!(!apply(&Operator::Between, &json!(0), Some(&json!([1, 10])), None));
    }

    #[test]
    fn is_empty_variants() {
        assert!(apply(&Operator::IsEmpty, &json!(null), None, None));
        assert!(apply(&Operator::IsEmpty, &json!(""), None, None));
        assert!(apply(&Operator::IsEmpty, &json!([]), None, None));
        assert!(!apply(&Operator::IsEmpty, &json!([1]), None, None));
    }

    #[test]
    fn in_list() {
        assert!(apply(&Operator::In, &json!("admin"), Some(&json!(["admin","editor"])), None));
        assert!(!apply(&Operator::In, &json!("viewer"), Some(&json!(["admin","editor"])), None));
    }
}
