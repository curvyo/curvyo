//! Expected IDs for fixtures written before four initial page-register operations.
//!
//! Only node identities shift; every geometry, style, order and reference remains
//! asserted. Anchor IDs are hex strings and are deliberately unaffected.
#![allow(dead_code, clippy::unwrap_used)]
use curvyo_document_core::NodeId;
use serde::Serialize;
use serde_json::Value;

pub fn value(expected: &impl Serialize) -> Value {
    fn shift(value: &mut Value) {
        match value {
            Value::Object(map)
                if map.len() == 2
                    && map.get("peer").is_some_and(Value::is_string)
                    && map.contains_key("counter") =>
            {
                let counter = map["counter"].as_i64().unwrap();
                assert!(
                    counter >= 4,
                    "expected a newly allocated post-pages node ID"
                );
                map.insert("counter".into(), Value::from(counter - 4));
            }
            Value::Object(map) => {
                for v in map.values_mut() {
                    shift(v);
                }
            }
            Value::Array(values) => {
                for v in values {
                    shift(v);
                }
            }
            _ => {}
        }
    }
    let mut result = serde_json::to_value(expected).unwrap();
    shift(&mut result);
    result
}

pub fn node(expected: NodeId) -> NodeId {
    serde_json::from_value(value(&expected)).unwrap()
}
