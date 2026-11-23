//! A small, dependency-free JSON reader for the recipe files this package parses.
//!
//! The registry forbids a registry dependency (R9: `Cargo.toml` may carry only the SDK's path
//! dependency), so `serde_json` is gone and this reader replaces the one thing the port used it
//! for. The seven machine recipe parsers need only the flat subset a data pack writes: an object,
//! an array, a string, an integer, `true`/`false`/`null`.
//!
//! [`Value::parse`] never panics: malformed text answers `None`, and every accessor defaults to
//! `None`, which is what each caller maps to its own `Unreadable` error — a recipe the reader
//! cannot understand is refused exactly as before, never trusted by accident.

use std::collections::BTreeMap;
use std::fmt;

/// The deepest nesting the reader accepts. A recipe nests three levels at most, so this only
/// stops a pathological file from exhausting the stack.
const MAX_DEPTH: u8 = 64;

/// A JSON value, as much of one as a recipe file uses.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// An integer that fits an `i64` (every number in a recipe file does).
    Number(i64),
    /// A number with a fraction or an exponent.
    Float(f64),
    /// A string, with its escapes decoded.
    String(String),
    /// An array, in file order.
    Array(Vec<Value>),
    /// An object; keys are sorted, because lookup is by name and never by position.
    Object(BTreeMap<String, Value>),
}

impl Eq for Value {}
// `Eq` is sound here because the reader refuses a non-finite float (`Value::parse` filters them),
// so the only `f64` this type holds is a real number, and `f64`'s `PartialEq` is reflexive on
// those. The parsers read integers; the float arm exists only so a file's `experience: 0.5` does
// not make the whole recipe unreadable.

impl Value {
    /// Parse one JSON document, or `None` when the text is malformed or nests past the limit.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let mut reader = JsonReader { text, at: 0 };
        let value = reader.value(1)?;
        reader.space();
        (reader.at == text.len()).then_some(value)
    }

    /// The member `key` of an object, or `None` for anything else.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(members) => members.get(key),
            _ => None,
        }
    }

    /// The string, when this is one.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    /// The array, when this is one.
    #[must_use]
    pub fn as_array(&self) -> Option<&Vec<Self>> {
        match self {
            Self::Array(items) => Some(items),
            _ => None,
        }
    }

    /// The object's members, when this is one.
    #[must_use]
    pub fn as_object(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Object(members) => Some(members),
            _ => None,
        }
    }

    /// The integer, when this is one that fits an `i64`.
    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// The non-negative integer, when this is one.
    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(number) => u64::try_from(*number).ok(),
            _ => None,
        }
    }

    /// The boolean, when this is one.
    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// Whether this is a string.
    #[must_use]
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Whether this is an array.
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    /// Whether this is `null`.
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// A missing member or an out-of-range index answers `null`, as `serde_json`'s indexing did for
/// the present members this port reads; it never panics on a file it half-understood.
static NULL: Value = Value::Null;

impl std::ops::Index<&str> for Value {
    type Output = Self;

    fn index(&self, key: &str) -> &Self {
        self.get(key).unwrap_or(&NULL)
    }
}

impl std::ops::Index<usize> for Value {
    type Output = Self;

    fn index(&self, index: usize) -> &Self {
        self.as_array()
            .and_then(|items| items.get(index))
            .unwrap_or(&NULL)
    }
}

impl PartialEq<str> for Value {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == Some(other)
    }
}

impl PartialEq<&str> for Value {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

impl PartialEq<String> for Value {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == Some(other.as_str())
    }
}

impl PartialEq<i32> for Value {
    fn eq(&self, other: &i32) -> bool {
        self.as_i64() == Some(i64::from(*other))
    }
}

impl PartialEq<u32> for Value {
    fn eq(&self, other: &u32) -> bool {
        self.as_i64() == Some(i64::from(*other))
    }
}

impl PartialEq<i64> for Value {
    fn eq(&self, other: &i64) -> bool {
        self.as_i64() == Some(*other)
    }
}

impl PartialEq<u64> for Value {
    fn eq(&self, other: &u64) -> bool {
        self.as_u64() == Some(*other)
    }
}

impl PartialEq<usize> for Value {
    fn eq(&self, other: &usize) -> bool {
        u64::try_from(*other)
            .ok()
            .and_then(|n| self.as_u64().map(|v| v == n))
            == Some(true)
    }
}

impl PartialEq<bool> for Value {
    fn eq(&self, other: &bool) -> bool {
        self.as_bool() == Some(*other)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("null"),
            Self::Bool(value) => write!(f, "{value}"),
            Self::Number(value) => write!(f, "{value}"),
            Self::Float(value) => write!(f, "{value}"),
            Self::String(text) => write_string(f, text),
            Self::Array(items) => {
                f.write_str("[")?;
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        f.write_str(",")?;
                    }
                    write!(f, "{item}")?;
                }
                f.write_str("]")
            }
            Self::Object(members) => {
                f.write_str("{")?;
                for (index, (key, value)) in members.iter().enumerate() {
                    if index > 0 {
                        f.write_str(",")?;
                    }
                    write_string(f, key)?;
                    write!(f, ":{value}")?;
                }
                f.write_str("}")
            }
        }
    }
}

/// Write `text` as a JSON string literal.
fn write_string(f: &mut fmt::Formatter<'_>, text: &str) -> fmt::Result {
    f.write_str("\"")?;
    for ch in text.chars() {
        match ch {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            ch if ch < ' ' => write!(f, "\\u{:04x}", u32::from(ch))?,
            ch => write!(f, "{ch}")?,
        }
    }
    f.write_str("\"")
}

/// A reader over the document's bytes. Positions always sit on an ASCII character when a number or
/// a literal is read, so slicing `text` at them is valid.
struct JsonReader<'a> {
    text: &'a str,
    at: usize,
}

impl JsonReader<'_> {
    /// Skip JSON's four whitespace characters.
    fn space(&mut self) {
        while self
            .peek()
            .is_some_and(|byte| matches!(byte, b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.at += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn take(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.at += 1;
        Some(byte)
    }

    fn expect(&mut self, want: u8) -> Option<()> {
        (self.take()? == want).then_some(())
    }

    fn value(&mut self, depth: u8) -> Option<Value> {
        if depth > MAX_DEPTH {
            return None;
        }
        self.space();
        match self.peek()? {
            b'{' => self.object(depth),
            b'[' => self.array(depth),
            b'"' => self.string().map(Value::String),
            b't' => self.literal("true", Value::Bool(true)),
            b'f' => self.literal("false", Value::Bool(false)),
            b'n' => self.literal("null", Value::Null),
            b'-' | b'0'..=b'9' => self.number(),
            _ => None,
        }
    }

    fn literal(&mut self, text: &str, value: Value) -> Option<Value> {
        self.text[self.at..].starts_with(text).then(|| {
            self.at += text.len();
            value
        })
    }

    fn object(&mut self, depth: u8) -> Option<Value> {
        self.expect(b'{')?;
        let mut members = BTreeMap::new();
        self.space();
        if self.peek()? == b'}' {
            self.at += 1;
            return Some(Value::Object(members));
        }
        loop {
            self.space();
            let key = self.string()?;
            self.space();
            self.expect(b':')?;
            let value = self.value(depth + 1)?;
            members.insert(key, value);
            self.space();
            match self.take()? {
                b',' => self.space(),
                b'}' => return Some(Value::Object(members)),
                _ => return None,
            }
        }
    }

    fn array(&mut self, depth: u8) -> Option<Value> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.space();
        if self.peek()? == b']' {
            self.at += 1;
            return Some(Value::Array(items));
        }
        loop {
            items.push(self.value(depth + 1)?);
            self.space();
            match self.take()? {
                b',' => self.space(),
                b']' => return Some(Value::Array(items)),
                _ => return None,
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            match self.take()? {
                b'"' => return Some(out),
                b'\\' => match self.take()? {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => out.push(self.hex_escape()?),
                    _ => return None,
                },
                byte if byte < 0x20 => return None,
                byte if byte.is_ascii() => out.push(char::from(byte)),
                _ => {
                    // The start of a multi-byte character: retry from its lead byte as a `char`.
                    self.at -= 1;
                    let ch = self.text[self.at..].chars().next()?;
                    out.push(ch);
                    self.at += ch.len_utf8();
                }
            }
        }
    }

    /// A `\uXXXX` escape, decoding a surrogate pair when one follows.
    fn hex_escape(&mut self) -> Option<char> {
        let high = self.hex4()?;
        if (0xD800..=0xDBFF).contains(&high) {
            self.expect(b'\\')?;
            self.expect(b'u')?;
            let low = self.hex4()?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return None;
            }
            let scalar = 0x1_0000 + ((high - 0xD800) << 10) + (low - 0xDC00);
            char::from_u32(scalar)
        } else {
            char::from_u32(high)
        }
    }

    fn hex4(&mut self) -> Option<u32> {
        let mut code = 0;
        for _ in 0..4 {
            code = code * 16 + char::from(self.take()?).to_digit(16)?;
        }
        Some(code)
    }

    fn number(&mut self) -> Option<Value> {
        let start = self.at;
        if self.peek() == Some(b'-') {
            self.at += 1;
        }
        let integer = self.at;
        self.digits();
        if self.at == integer {
            return None;
        }
        let mut float = false;
        if self.peek() == Some(b'.') {
            float = true;
            self.at += 1;
            let fraction = self.at;
            self.digits();
            if self.at == fraction {
                return None;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            float = true;
            self.at += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.at += 1;
            }
            let exponent = self.at;
            self.digits();
            if self.at == exponent {
                return None;
            }
        }
        let text = &self.text[start..self.at];
        if float {
            text.parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(Value::Float)
        } else {
            text.parse::<i64>().ok().map(Value::Number)
        }
    }

    fn digits(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            self.at += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pulverizer file, as the data pack writes it (the shape the seven parsers read).
    const BONE: &str = r#"{
        "type": "oritech:pulverizer",
        "ingredients": [ "minecraft:bone" ],
        "results": [ { "id": "minecraft:bone_meal", "count": 6 } ],
        "time": 100
    }"#;

    fn json(text: &str) -> Value {
        Value::parse(text).unwrap_or_else(|| panic!("{text} is JSON"))
    }

    #[test]
    fn a_pulverizer_file_reads_its_type_ingredients_results_and_time() {
        let recipe = json(BONE);
        assert_eq!(recipe["type"], "oritech:pulverizer");
        assert_eq!(
            recipe.get("type").and_then(Value::as_str),
            Some("oritech:pulverizer")
        );
        let ingredients = recipe["ingredients"]
            .as_array()
            .expect("an ingredient list");
        assert_eq!(ingredients.len(), 1);
        assert_eq!(ingredients[0], "minecraft:bone");
        let results = recipe["results"].as_array().expect("a result list");
        assert_eq!(results[0].get("count").and_then(Value::as_i64), Some(6));
        assert_eq!(recipe["time"].as_u64(), Some(100));
        assert_eq!(recipe["time"], 100);
        assert!(recipe["results"].is_array());
        assert!(!recipe["results"].is_string());
    }

    #[test]
    fn a_float_member_does_not_make_the_document_unreadable() {
        // Vanilla smelting files carry `experience`; the port never reads it, but it must parse.
        let smelting = json(
            r#"{"cookingtime": 200, "experience": 0.5, "result": {"id": "minecraft:iron_ingot"}}"#,
        );
        assert_eq!(smelting["cookingtime"], 200);
        assert_eq!(smelting["experience"].as_i64(), None);
        assert_eq!(smelting["experience"], Value::Float(0.5));
    }

    #[test]
    fn escapes_and_unicode_are_decoded() {
        let value = json(
            r#"{"a": "line\nquote\"slash\\tab\t", "u": "\u00e9\u4e16", "pair": "\ud83d\ude00"}"#,
        );
        assert_eq!(value["a"], "line\nquote\"slash\\tab\t");
        assert_eq!(value["u"], "é世");
        assert_eq!(value["pair"], "😀");
    }

    #[test]
    fn an_absent_member_or_index_is_null_and_never_panics() {
        let value = json(r#"{"list": [1, 2]}"#);
        assert!(value["missing"].is_null());
        assert!(value["list"][7].is_null());
        assert!(value.as_str().is_none());
        assert!(value.get("list").is_some());
        assert!(Value::Null.get("x").is_none());
    }

    #[test]
    fn a_malformed_document_is_refused_rather_than_trusted() {
        for text in [
            "",
            "   ",
            "{",
            "{\"a\":}",
            "{\"a\": 1,}",
            "[1,]",
            "{\"a\": 1} trailing",
            "{'a': 1}",
            "\"unterminated",
            "\"bad escape \\q\"",
            "-",
            "1.",
            "1e",
            "tru",
        ] {
            assert!(Value::parse(text).is_none(), "{text:?} is not JSON");
        }
    }

    #[test]
    fn nesting_past_the_limit_is_refused_and_never_exhausts_the_stack() {
        let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
        assert!(Value::parse(&deep).is_none());
        let allowed = format!("{}{}", "[".repeat(60), "]".repeat(60));
        assert!(Value::parse(&allowed).is_some());
    }

    #[test]
    fn display_writes_a_document_that_parses_back_to_the_same_value() {
        let value = json(BONE);
        let written = value.to_string();
        assert_eq!(Value::parse(&written).as_ref(), Some(&value));
        assert_eq!(
            Value::String("a\"b\n".to_owned()).to_string(),
            "\"a\\\"b\\n\""
        );
    }
}
