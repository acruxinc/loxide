//! A minimal, dependency-free JSON value type and serializer.
//!
//! loxide does not depend on `serde` or `serde_json`. Instead it ships this
//! small [`JsonValue`] type, which is enough to represent structured log
//! fields and serialize them to RFC 8259-compliant JSON.
//!
//! Values are usually constructed with the [`json!`](crate::json!) macro or via
//! the many [`From`] implementations:
//!
//! ```
//! use loxide::JsonValue;
//!
//! let a: JsonValue = "hello".into();
//! let b: JsonValue = 42.into();
//! let c = loxide::json!(true);
//!
//! assert_eq!(a.to_string(), "\"hello\"");
//! assert_eq!(b.to_string(), "42");
//! assert_eq!(c.to_string(), "true");
//! ```

use std::collections::HashMap;
use std::fmt;
use std::ops::Index;

/// A JSON number, stored as either a signed integer or a floating-point value.
#[derive(Debug, Clone)]
pub enum JsonNumber {
    /// A 64-bit signed integer.
    Int(i64),
    /// A 64-bit floating-point number.
    Float(f64),
}

impl PartialEq for JsonNumber {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (JsonNumber::Int(a), JsonNumber::Int(b)) => a == b,
            (JsonNumber::Float(a), JsonNumber::Float(b)) => a == b,
            (JsonNumber::Int(a), JsonNumber::Float(b)) => (*a as f64) == *b,
            (JsonNumber::Float(a), JsonNumber::Int(b)) => *a == (*b as f64),
        }
    }
}

/// A JSON value.
///
/// This is loxide's self-contained replacement for `serde_json::Value`. Object
/// entries preserve insertion order (they are stored as a `Vec` of pairs), which
/// keeps rendered log lines stable and predictable.
#[derive(Debug, Clone, Default)]
pub enum JsonValue {
    /// The JSON `null` literal.
    #[default]
    Null,
    /// A JSON boolean.
    Bool(bool),
    /// A JSON number.
    Number(JsonNumber),
    /// A JSON string.
    String(String),
    /// A JSON array.
    Array(Vec<JsonValue>),
    /// A JSON object, as an ordered list of key/value pairs.
    Object(Vec<(String, JsonValue)>),
}

impl PartialEq for JsonValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (JsonValue::Null, JsonValue::Null) => true,
            (JsonValue::Bool(a), JsonValue::Bool(b)) => a == b,
            (JsonValue::Number(a), JsonValue::Number(b)) => a == b,
            (JsonValue::String(a), JsonValue::String(b)) => a == b,
            (JsonValue::Array(a), JsonValue::Array(b)) => a == b,
            (JsonValue::Object(a), JsonValue::Object(b)) => a == b,
            _ => false,
        }
    }
}

// --- Ergonomic comparison impls (mostly for assertions in tests) ---

impl PartialEq<&str> for JsonValue {
    fn eq(&self, other: &&str) -> bool {
        matches!(self, JsonValue::String(s) if s == other)
    }
}

impl PartialEq<str> for JsonValue {
    fn eq(&self, other: &str) -> bool {
        matches!(self, JsonValue::String(s) if s == other)
    }
}

impl PartialEq<bool> for JsonValue {
    fn eq(&self, other: &bool) -> bool {
        matches!(self, JsonValue::Bool(b) if b == other)
    }
}

impl PartialEq<i32> for JsonValue {
    fn eq(&self, other: &i32) -> bool {
        matches!(self, JsonValue::Number(JsonNumber::Int(n)) if *n == i64::from(*other))
    }
}

impl PartialEq<i64> for JsonValue {
    fn eq(&self, other: &i64) -> bool {
        matches!(self, JsonValue::Number(JsonNumber::Int(n)) if n == other)
    }
}

impl PartialEq<f64> for JsonValue {
    fn eq(&self, other: &f64) -> bool {
        match self {
            JsonValue::Number(JsonNumber::Float(f)) => f == other,
            JsonValue::Number(JsonNumber::Int(n)) => (*n as f64) == *other,
            _ => false,
        }
    }
}

// --- Accessors ---

impl JsonValue {
    /// Returns `true` if the value is [`JsonValue::Null`].
    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    /// Returns the contained string slice, or `None` if this is not a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Returns the contained boolean, or `None` if this is not a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the value as an `i64`, or `None` if this is not an integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(JsonNumber::Int(n)) => Some(*n),
            _ => None,
        }
    }

    /// Returns the value as an `f64`, accepting either integer or float numbers.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            JsonValue::Number(JsonNumber::Float(f)) => Some(*f),
            JsonValue::Number(JsonNumber::Int(n)) => Some(*n as f64),
            _ => None,
        }
    }

    /// Returns the contained slice if this is an array, otherwise `None`.
    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Looks up a key in an object value, returning `None` for missing keys or
    /// non-object values. Unlike indexing, this never panics.
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

/// Indexes into an object by key.
///
/// # Panics
///
/// Panics if `self` is not a [`JsonValue::Object`] or the key is absent. Use
/// [`JsonValue::get`] for a non-panicking lookup.
impl Index<&str> for JsonValue {
    type Output = JsonValue;

    fn index(&self, key: &str) -> &JsonValue {
        self.get(key)
            .unwrap_or_else(|| panic!("no such key in JSON object: {key:?}"))
    }
}

// --- Constructors via `From` ---

impl From<&str> for JsonValue {
    fn from(s: &str) -> Self {
        JsonValue::String(s.to_string())
    }
}

impl From<String> for JsonValue {
    fn from(s: String) -> Self {
        JsonValue::String(s)
    }
}

impl From<&String> for JsonValue {
    fn from(s: &String) -> Self {
        JsonValue::String(s.clone())
    }
}

impl From<bool> for JsonValue {
    fn from(b: bool) -> Self {
        JsonValue::Bool(b)
    }
}

/// Generates `From<$int>` impls that widen losslessly into [`JsonNumber::Int`].
macro_rules! impl_from_int {
    ($($t:ty),* $(,)?) => {$(
        impl From<$t> for JsonValue {
            fn from(n: $t) -> Self {
                JsonValue::Number(JsonNumber::Int(i64::from(n)))
            }
        }
    )*};
}
impl_from_int!(i8, i16, i32, i64, u8, u16, u32);

impl From<f32> for JsonValue {
    fn from(n: f32) -> Self {
        JsonValue::Number(JsonNumber::Float(f64::from(n)))
    }
}

impl From<f64> for JsonValue {
    fn from(n: f64) -> Self {
        JsonValue::Number(JsonNumber::Float(n))
    }
}

impl<T: Into<JsonValue>> From<Option<T>> for JsonValue {
    fn from(opt: Option<T>) -> Self {
        opt.map_or(JsonValue::Null, Into::into)
    }
}

impl<T: Into<JsonValue>> From<Vec<T>> for JsonValue {
    fn from(items: Vec<T>) -> Self {
        JsonValue::Array(items.into_iter().map(Into::into).collect())
    }
}

impl From<HashMap<String, String>> for JsonValue {
    fn from(map: HashMap<String, String>) -> Self {
        // Sort keys so serialized output is deterministic across runs.
        let mut pairs: Vec<(String, JsonValue)> = map
            .into_iter()
            .map(|(k, v)| (k, JsonValue::String(v)))
            .collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        JsonValue::Object(pairs)
    }
}

// --- Serialization ---

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonValue::Null => f.write_str("null"),
            JsonValue::Bool(true) => f.write_str("true"),
            JsonValue::Bool(false) => f.write_str("false"),
            JsonValue::Number(n) => fmt::Display::fmt(n, f),
            JsonValue::String(s) => write_json_string(f, s),
            JsonValue::Array(items) => {
                f.write_str("[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    fmt::Display::fmt(v, f)?;
                }
                f.write_str("]")
            }
            JsonValue::Object(pairs) => {
                f.write_str("{")?;
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        f.write_str(",")?;
                    }
                    write_json_string(f, k)?;
                    f.write_str(":")?;
                    fmt::Display::fmt(v, f)?;
                }
                f.write_str("}")
            }
        }
    }
}

impl fmt::Display for JsonNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JsonNumber::Int(n) => write!(f, "{n}"),
            // JSON has no representation for NaN/Infinity, so emit `null` to keep
            // the output valid rather than producing "NaN"/"inf".
            JsonNumber::Float(n) if n.is_finite() => write!(f, "{n}"),
            JsonNumber::Float(_) => f.write_str("null"),
        }
    }
}

/// Writes `s` as a quoted, escaped JSON string per RFC 8259.
fn write_json_string(f: &mut fmt::Formatter<'_>, s: &str) -> fmt::Result {
    f.write_str("\"")?;
    for c in s.chars() {
        match c {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\u{0008}' => f.write_str("\\b")?,
            '\u{000C}' => f.write_str("\\f")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            c if (c as u32) < 0x20 => write!(f, "\\u{:04x}", c as u32)?,
            c => f.write_str(c.encode_utf8(&mut [0u8; 4]))?,
        }
    }
    f.write_str("\"")
}

/// Constructs a [`JsonValue`] from any value implementing `Into<JsonValue>`.
///
/// This is loxide's lightweight stand-in for `serde_json::json!`. It is
/// deliberately simple — it forwards to the [`From`] implementations on
/// [`JsonValue`]:
///
/// ```
/// use loxide::{json, JsonValue};
///
/// let v = json!("hello");
/// assert_eq!(v, JsonValue::String("hello".to_string()));
/// assert_eq!(json!(7), JsonValue::from(7));
/// ```
#[macro_export]
macro_rules! json {
    ($e:expr) => {
        $crate::JsonValue::from($e)
    };
}

// --- Test-only JSON parser -------------------------------------------------
//
// A small recursive-descent parser used exclusively by the crate's own tests to
// verify serialized output. It is not part of the public API and is compiled
// only under `cfg(test)`.

#[cfg(test)]
pub(crate) fn from_json_str(input: &str) -> Option<JsonValue> {
    let bytes = input.trim().as_bytes();
    let (value, _) = parse_value(bytes, 0)?;
    Some(value)
}

#[cfg(test)]
fn skip_ws(input: &[u8], mut pos: usize) -> usize {
    while pos < input.len() && matches!(input[pos], b' ' | b'\t' | b'\n' | b'\r') {
        pos += 1;
    }
    pos
}

#[cfg(test)]
fn parse_value(input: &[u8], pos: usize) -> Option<(JsonValue, usize)> {
    let pos = skip_ws(input, pos);
    match input.get(pos)? {
        b'"' => parse_string(input, pos).map(|(s, p)| (JsonValue::String(s), p)),
        b'{' => parse_object(input, pos),
        b'[' => parse_array(input, pos),
        b't' | b'f' => parse_bool(input, pos),
        b'n' => parse_null(input, pos),
        b'-' | b'0'..=b'9' => parse_number(input, pos),
        _ => None,
    }
}

#[cfg(test)]
fn parse_string(input: &[u8], pos: usize) -> Option<(String, usize)> {
    if input.get(pos) != Some(&b'"') {
        return None;
    }
    let mut pos = pos + 1;
    let mut s = String::new();
    while let Some(&byte) = input.get(pos) {
        match byte {
            b'"' => return Some((s, pos + 1)),
            b'\\' => {
                pos += 1;
                match input.get(pos)? {
                    b'"' => s.push('"'),
                    b'\\' => s.push('\\'),
                    b'/' => s.push('/'),
                    b'b' => s.push('\u{0008}'),
                    b'f' => s.push('\u{000C}'),
                    b'n' => s.push('\n'),
                    b'r' => s.push('\r'),
                    b't' => s.push('\t'),
                    b'u' => {
                        let hex = std::str::from_utf8(input.get(pos + 1..pos + 5)?).ok()?;
                        let code = u32::from_str_radix(hex, 16).ok()?;
                        s.push(char::from_u32(code)?);
                        pos += 4;
                    }
                    _ => return None,
                }
                pos += 1;
            }
            b => {
                s.push(b as char);
                pos += 1;
            }
        }
    }
    None
}

#[cfg(test)]
fn parse_number(input: &[u8], start: usize) -> Option<(JsonValue, usize)> {
    let mut pos = start;
    let mut is_float = false;

    if input.get(pos) == Some(&b'-') {
        pos += 1;
    }
    while input.get(pos).is_some_and(u8::is_ascii_digit) {
        pos += 1;
    }
    if input.get(pos) == Some(&b'.') {
        is_float = true;
        pos += 1;
        while input.get(pos).is_some_and(u8::is_ascii_digit) {
            pos += 1;
        }
    }
    if matches!(input.get(pos), Some(b'e' | b'E')) {
        is_float = true;
        pos += 1;
        if matches!(input.get(pos), Some(b'+' | b'-')) {
            pos += 1;
        }
        while input.get(pos).is_some_and(u8::is_ascii_digit) {
            pos += 1;
        }
    }

    let s = std::str::from_utf8(&input[start..pos]).ok()?;
    let number = if is_float {
        JsonNumber::Float(s.parse().ok()?)
    } else {
        JsonNumber::Int(s.parse().ok()?)
    };
    Some((JsonValue::Number(number), pos))
}

#[cfg(test)]
fn parse_object(input: &[u8], pos: usize) -> Option<(JsonValue, usize)> {
    let mut pos = skip_ws(input, pos + 1);
    let mut pairs = Vec::new();

    if input.get(pos) == Some(&b'}') {
        return Some((JsonValue::Object(pairs), pos + 1));
    }

    loop {
        pos = skip_ws(input, pos);
        let (key, p) = parse_string(input, pos)?;
        pos = skip_ws(input, p);
        if input.get(pos) != Some(&b':') {
            return None;
        }
        pos = skip_ws(input, pos + 1);
        let (value, p) = parse_value(input, pos)?;
        pairs.push((key, value));
        pos = skip_ws(input, p);
        match input.get(pos)? {
            b'}' => return Some((JsonValue::Object(pairs), pos + 1)),
            b',' => pos += 1,
            _ => return None,
        }
    }
}

#[cfg(test)]
fn parse_array(input: &[u8], pos: usize) -> Option<(JsonValue, usize)> {
    let mut pos = skip_ws(input, pos + 1);
    let mut items = Vec::new();

    if input.get(pos) == Some(&b']') {
        return Some((JsonValue::Array(items), pos + 1));
    }

    loop {
        pos = skip_ws(input, pos);
        let (value, p) = parse_value(input, pos)?;
        items.push(value);
        pos = skip_ws(input, p);
        match input.get(pos)? {
            b']' => return Some((JsonValue::Array(items), pos + 1)),
            b',' => pos += 1,
            _ => return None,
        }
    }
}

#[cfg(test)]
fn parse_bool(input: &[u8], pos: usize) -> Option<(JsonValue, usize)> {
    if input[pos..].starts_with(b"true") {
        Some((JsonValue::Bool(true), pos + 4))
    } else if input[pos..].starts_with(b"false") {
        Some((JsonValue::Bool(false), pos + 5))
    } else {
        None
    }
}

#[cfg(test)]
fn parse_null(input: &[u8], pos: usize) -> Option<(JsonValue, usize)> {
    input[pos..]
        .starts_with(b"null")
        .then_some((JsonValue::Null, pos + 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_scalars() {
        assert_eq!(JsonValue::Null.to_string(), "null");
        assert_eq!(json!(true).to_string(), "true");
        assert_eq!(json!(42).to_string(), "42");
        assert_eq!(json!("hi").to_string(), "\"hi\"");
    }

    #[test]
    fn escapes_strings() {
        let v = JsonValue::String("a\"b\\c\nd\te".to_string());
        assert_eq!(v.to_string(), "\"a\\\"b\\\\c\\nd\\te\"");
    }

    #[test]
    fn escapes_control_characters() {
        let v = JsonValue::String("\u{0001}".to_string());
        assert_eq!(v.to_string(), "\"\\u0001\"");
    }

    #[test]
    fn non_finite_floats_serialize_as_null() {
        assert_eq!(json!(f64::NAN).to_string(), "null");
        assert_eq!(json!(f64::INFINITY).to_string(), "null");
    }

    #[test]
    fn serializes_nested_structures() {
        let v = JsonValue::Object(vec![
            ("nums".to_string(), JsonValue::from(vec![1, 2, 3])),
            ("ok".to_string(), json!(true)),
        ]);
        assert_eq!(v.to_string(), "{\"nums\":[1,2,3],\"ok\":true}");
    }

    #[test]
    fn accessors_work() {
        assert_eq!(json!("x").as_str(), Some("x"));
        assert_eq!(json!(5).as_i64(), Some(5));
        assert_eq!(json!(5).as_f64(), Some(5.0));
        assert_eq!(json!(true).as_bool(), Some(true));
        assert!(JsonValue::Null.is_null());
    }

    #[test]
    fn get_and_index() {
        let v = JsonValue::Object(vec![("k".to_string(), json!("v"))]);
        assert_eq!(v.get("k"), Some(&json!("v")));
        assert_eq!(v.get("missing"), None);
        assert_eq!(v["k"], "v");
    }

    #[test]
    fn option_and_vec_conversions() {
        let some: JsonValue = Some(3).into();
        let none: JsonValue = Option::<i32>::None.into();
        assert_eq!(some, 3);
        assert!(none.is_null());
        assert_eq!(JsonValue::from(vec!["a", "b"]).to_string(), "[\"a\",\"b\"]");
    }

    #[test]
    fn round_trips_through_parser() {
        let original = "{\"a\":1,\"b\":[true,null,\"x\"],\"c\":1.5}";
        let parsed = from_json_str(original).unwrap();
        assert_eq!(parsed["a"], 1);
        assert_eq!(parsed["b"].as_array().unwrap().len(), 3);
        assert_eq!(parsed["c"], 1.5);
    }
}
