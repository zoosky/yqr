//! yqr's native value model — decoupled from any YAML backend library.
//!
//! The evaluator ([`crate::eval`]) operates on this [`Value`], not on the
//! parser's own value type. The YAML engine (noyalib) is converted **into**
//! `Value` at parse time (`From<noyalib::Value>`) and **back** at emit time
//! (`From<&Value> for noyalib::Value`), so swapping or upgrading the backend is
//! an internal change that never touches the evaluator, the public API, or the
//! tests written against `Value`.
//!
//! The shape deliberately mirrors the model yqr's evaluator has always used:
//! distinct `Int`/`Float` scalars and an insertion-ordered, `Value`-keyed
//! [`Mapping`] (so non-string keys are representable and key order is stable).

use std::hash::{Hash, Hasher};

use indexmap::IndexMap;

/// An insertion-ordered YAML mapping keyed by [`Value`].
///
/// Keys may be any scalar (or, in principle, any `Value`); order is preserved,
/// matching how the source document was written.
pub type Mapping = IndexMap<Value, Value>;

/// A YAML value in yqr's evaluation model.
///
/// This is yqr's own type, independent of the YAML parser. Integers and floats
/// are kept distinct (jq-style numeric semantics), and mappings preserve key
/// order.
#[derive(Debug, Clone, Default)]
pub enum Value {
    /// YAML `null`.
    #[default]
    Null,
    /// A boolean.
    Bool(bool),
    /// A signed 64-bit integer.
    Int(i64),
    /// A 64-bit floating-point number.
    Float(f64),
    /// A string scalar.
    String(String),
    /// A sequence (array).
    Sequence(Vec<Value>),
    /// A mapping (object); key insertion order is preserved.
    Mapping(Mapping),
}

// `Value` is used as an `IndexMap` key, so it needs `Eq` + `Hash`. Floats are
// compared and hashed by their bit pattern, which keeps `Eq` reflexive (so
// `NaN == NaN` here) and consistent with `Hash` — the invariant a map key
// requires. This never affects arithmetic (yqr does none); it only governs key
// identity.
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Sequence(a), Value::Sequence(b)) => a == b,
            (Value::Mapping(a), Value::Mapping(b)) => a == b,
            _ => false,
        }
    }
}

impl Value {
    /// If this is a mapping containing string key `key`, return its value.
    ///
    /// Convenience for the common "look up a named field on an object" case;
    /// returns `None` for non-mappings or absent keys.
    #[must_use]
    pub fn get_str(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Mapping(map) => map.get(&Value::String(key.to_string())),
            _ => None,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Bool(b) => b.hash(state),
            Value::Int(i) => i.hash(state),
            Value::Float(f) => f.to_bits().hash(state),
            Value::String(s) => s.hash(state),
            Value::Sequence(items) => items.hash(state),
            Value::Mapping(map) => {
                for (k, v) in map {
                    k.hash(state);
                    v.hash(state);
                }
            }
        }
    }
}

/// Lower the YAML engine's value into yqr's model (parse boundary).
///
/// The typed view is intentionally lossy: noyalib tags collapse to their inner
/// value, an unsigned integer above `i64::MAX` degrades to a float, and its
/// string-only mapping keys become [`Value::String`] keys. Byte fidelity is
/// never derived from this view (that is the fidelity engine's job).
impl From<&noyalib::Value> for Value {
    fn from(v: &noyalib::Value) -> Self {
        match v {
            noyalib::Value::Null => Value::Null,
            noyalib::Value::Bool(b) => Value::Bool(*b),
            noyalib::Value::Number(n) => n
                .as_i64()
                .map_or_else(|| Value::Float(n.as_f64()), Value::Int),
            noyalib::Value::String(s) => Value::String(s.clone()),
            noyalib::Value::Sequence(items) => {
                Value::Sequence(items.iter().map(Value::from).collect())
            }
            noyalib::Value::Mapping(map) => Value::Mapping(
                map.iter()
                    .map(|(k, v)| (Value::String(k.clone()), Value::from(v)))
                    .collect(),
            ),
            noyalib::Value::Tagged(t) => Value::from(t.value()),
        }
    }
}

impl From<noyalib::Value> for Value {
    fn from(v: noyalib::Value) -> Self {
        Value::from(&v)
    }
}

/// Raise a yqr value back into the engine's model for emission (emit boundary).
///
/// noyalib mappings are string-keyed, so a non-string key is emitted via its
/// scalar spelling; a composite key (sequence/mapping) has no string form and
/// is dropped from the emitted mapping. yqr only emits *computed* or *absent*
/// results this way — faithful reads are sliced from source by the fidelity
/// engine and never round-trip through here.
impl From<&Value> for noyalib::Value {
    fn from(v: &Value) -> Self {
        match v {
            Value::Null => noyalib::Value::Null,
            Value::Bool(b) => noyalib::Value::Bool(*b),
            Value::Int(i) => noyalib::Value::from(*i),
            Value::Float(f) => noyalib::Value::from(*f),
            Value::String(s) => noyalib::Value::String(s.clone()),
            Value::Sequence(items) => {
                noyalib::Value::Sequence(items.iter().map(noyalib::Value::from).collect())
            }
            Value::Mapping(map) => {
                let mut out = noyalib::Mapping::new();
                for (k, val) in map {
                    let key = match k {
                        Value::String(s) => s.clone(),
                        Value::Int(i) => i.to_string(),
                        Value::Float(f) => f.to_string(),
                        Value::Bool(b) => b.to_string(),
                        Value::Null => "null".to_string(),
                        // Composite keys have no string spelling; skip them.
                        Value::Sequence(_) | Value::Mapping(_) => continue,
                    };
                    out.insert(key, noyalib::Value::from(val));
                }
                noyalib::Value::Mapping(out)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_scalars_through_noyalib() {
        for src in ["null", "true", "42", "1.5", "\"hi\""] {
            let ny = noyalib::from_str::<noyalib::Value>(src).unwrap();
            let v = Value::from(&ny);
            let back = noyalib::Value::from(&v);
            assert_eq!(
                Value::from(&back),
                v,
                "scalar `{src}` survived the round trip"
            );
        }
    }

    #[test]
    fn distinguishes_int_and_float() {
        assert_eq!(
            Value::from(&noyalib::from_str::<noyalib::Value>("7").unwrap()),
            Value::Int(7)
        );
        assert_eq!(
            Value::from(&noyalib::from_str::<noyalib::Value>("7.0").unwrap()),
            Value::Float(7.0)
        );
    }

    #[test]
    fn mapping_preserves_key_order() {
        let ny = noyalib::from_str::<noyalib::Value>("z: 1\na: 2\n").unwrap();
        let Value::Mapping(map) = Value::from(&ny) else {
            panic!("expected mapping");
        };
        let keys: Vec<_> = map.keys().cloned().collect();
        assert_eq!(
            keys,
            vec![Value::String("z".into()), Value::String("a".into())]
        );
    }
}

/// Serde impls for [`Value`], behind the `serde` cargo feature.
///
/// Unlike the validator's diagnostics (outputs, `Serialize` only), a value
/// travels both ways: the typed edit API's ops carry one *into* yqr, so
/// `Deserialize` exists too. The wire shape is the JSON data model — a
/// mapping serializes as an object, so a non-string key refuses rather
/// than being silently renamed; deserialized object keys come back as
/// [`Value::String`] keys. An integer deserializes as [`Value::Int`] when
/// it fits `i64` and as [`Value::Float`] otherwise.
// Feature f050.
#[cfg(feature = "serde")]
mod serde_impls {
    use super::{Mapping, Value};
    use serde::de::{MapAccess, SeqAccess, Visitor};
    use serde::ser::{Error as _, SerializeMap, SerializeSeq};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::fmt;

    impl Serialize for Value {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            match self {
                Value::Null => s.serialize_unit(),
                Value::Bool(b) => s.serialize_bool(*b),
                Value::Int(n) => s.serialize_i64(*n),
                Value::Float(f) => s.serialize_f64(*f),
                Value::String(text) => s.serialize_str(text),
                Value::Sequence(items) => {
                    let mut seq = s.serialize_seq(Some(items.len()))?;
                    for item in items {
                        seq.serialize_element(item)?;
                    }
                    seq.end()
                }
                Value::Mapping(map) => {
                    let mut out = s.serialize_map(Some(map.len()))?;
                    for (key, value) in map {
                        let Value::String(key) = key else {
                            return Err(S::Error::custom(
                                "a mapping key is not a string, so the value has no \
                                 JSON-object spelling",
                            ));
                        };
                        out.serialize_entry(key, value)?;
                    }
                    out.end()
                }
            }
        }
    }

    impl<'de> Deserialize<'de> for Value {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct ValueVisitor;

            impl<'de> Visitor<'de> for ValueVisitor {
                type Value = Value;

                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("a JSON-model value")
                }

                fn visit_unit<E>(self) -> Result<Value, E> {
                    Ok(Value::Null)
                }

                fn visit_none<E>(self) -> Result<Value, E> {
                    Ok(Value::Null)
                }

                fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
                    Value::deserialize(d)
                }

                fn visit_bool<E>(self, b: bool) -> Result<Value, E> {
                    Ok(Value::Bool(b))
                }

                fn visit_i64<E>(self, n: i64) -> Result<Value, E> {
                    Ok(Value::Int(n))
                }

                fn visit_u64<E>(self, n: u64) -> Result<Value, E> {
                    Ok(i64::try_from(n).map_or(Value::Float(n as f64), Value::Int))
                }

                fn visit_f64<E>(self, f: f64) -> Result<Value, E> {
                    Ok(Value::Float(f))
                }

                fn visit_str<E>(self, text: &str) -> Result<Value, E> {
                    Ok(Value::String(text.to_owned()))
                }

                fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                    let mut items = Vec::new();
                    while let Some(item) = seq.next_element()? {
                        items.push(item);
                    }
                    Ok(Value::Sequence(items))
                }

                fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Value, A::Error> {
                    let mut map = Mapping::new();
                    while let Some((key, value)) = access.next_entry::<String, Value>()? {
                        map.insert(Value::String(key), value);
                    }
                    Ok(Value::Mapping(map))
                }
            }

            d.deserialize_any(ValueVisitor)
        }
    }
}
