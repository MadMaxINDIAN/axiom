use serde_json::Value;

/// Walk a dot-notation field path (with optional array indexing) into a
/// `serde_json::Value`, returning `Value::Null` for missing paths rather than
/// an error.  This matches the ARS §4.4 spec.
///
/// Examples:
/// - `"applicant.credit_score"` → nested object lookup
/// - `"order.items[0].price"` → array index then object lookup
pub fn resolve_path<'a>(context: &'a Value, path: &str) -> &'a Value {
    let mut current = context;
    for segment in split_path(path) {
        current = step(current, &segment);
        if current.is_null() {
            return current;
        }
    }
    current
}

/// Returns an owned resolved value (clones only what is needed).
pub fn resolve_owned(context: &Value, path: &str) -> Value {
    resolve_path(context, path).clone()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum Segment {
    Key(String),
    Index(usize),
}

fn split_path(path: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    for part in path.split('.') {
        // Handle `key[0]` or `key[0][1]` within a single dot-segment.
        let mut remaining = part;
        while let Some(bracket_pos) = remaining.find('[') {
            let key = &remaining[..bracket_pos];
            if !key.is_empty() {
                segments.push(Segment::Key(key.to_string()));
            }
            remaining = &remaining[bracket_pos + 1..];
            if let Some(close) = remaining.find(']') {
                let idx: usize = remaining[..close].parse().unwrap_or(0);
                segments.push(Segment::Index(idx));
                remaining = &remaining[close + 1..];
                // skip leading `[` for chained indices handled in next loop iteration
                remaining = remaining.trim_start_matches('.');
            } else {
                break;
            }
        }
        if !remaining.is_empty() {
            segments.push(Segment::Key(remaining.to_string()));
        }
    }
    segments
}

fn step<'a>(node: &'a Value, segment: &Segment) -> &'a Value {
    match segment {
        Segment::Key(k) => node.get(k).unwrap_or(&Value::Null),
        Segment::Index(i) => node.get(i).unwrap_or(&Value::Null),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn simple_nested_path() {
        let ctx = json!({ "applicant": { "credit_score": 720 } });
        assert_eq!(resolve_path(&ctx, "applicant.credit_score"), &json!(720));
    }

    #[test]
    fn array_index() {
        let ctx = json!({ "order": { "items": [{"price": 9.99}] } });
        assert_eq!(resolve_path(&ctx, "order.items[0].price"), &json!(9.99));
    }

    #[test]
    fn missing_path_returns_null() {
        let ctx = json!({ "a": 1 });
        assert_eq!(resolve_path(&ctx, "b.c"), &Value::Null);
    }

    #[test]
    fn top_level_key() {
        let ctx = json!({ "score": 42 });
        assert_eq!(resolve_path(&ctx, "score"), &json!(42));
    }
}
