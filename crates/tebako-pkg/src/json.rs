//! Minimal JSON for manifest.json: an exact-format writer (mirrors the C++
//! `package.cpp` output byte for byte) and a small recursive parser
//! (mirrors the C++ `JsonParser` semantics for the fields reassemble
//! needs). No serde — the wire format is fixed and tiny.

/// Escape a string for JSON output (mirrors the C++ `json_escape`).
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// A parsed JSON value (only what reassemble needs).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    /// Object member lookup.
    pub fn find(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// As u64 (only from Number, full parse — mirrors the C++ as_u64).
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// As string (only from String).
    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }
}

/// Parse a JSON text (mirrors the C++ parser: single top-level value,
/// trailing characters rejected). Errors are short human strings.
pub fn parse(text: &str) -> Result<Value, String> {
    let mut p = Parser {
        s: text.as_bytes(),
        pos: 0,
    };
    p.skip_ws();
    let v = p.value()?;
    p.skip_ws();
    if p.pos != p.s.len() {
        return Err("trailing characters after JSON value".into());
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    pos: usize,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.pos < self.s.len() && self.s[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.pos).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() != Some(c) {
            return Err(format!("expected '{}'", c as char));
        }
        self.pos += 1;
        Ok(())
    }

    fn starts_with(&self, lit: &str) -> bool {
        self.s[self.pos..].starts_with(lit.as_bytes())
    }

    fn value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek() {
            None => Err("unexpected end of input".into()),
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Value::String(self.string()?)),
            _ if self.starts_with("true") => {
                self.pos += 4;
                Ok(Value::Bool(true))
            }
            _ if self.starts_with("false") => {
                self.pos += 5;
                Ok(Value::Bool(false))
            }
            _ if self.starts_with("null") => {
                self.pos += 4;
                Ok(Value::Null)
            }
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            _ => Err("unexpected character in JSON input".into()),
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.pos += 1; // '{'
        let mut members = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            let val = self.value()?;
            members.push((key, val));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err("expected ',' or '}' in object".into()),
            }
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.pos += 1; // '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err("expected ',' or ']' in array".into()),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err("unterminated string".into());
            };
            self.pos += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let Some(esc) = self.peek() else {
                        return Err("unterminated string".into());
                    };
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            if self.pos + 4 > self.s.len() {
                                return Err("truncated \\u escape".into());
                            }
                            let hex = std::str::from_utf8(&self.s[self.pos..self.pos + 4])
                                .map_err(|_| "invalid \\u escape".to_string())?;
                            let code = u32::from_str_radix(hex, 16)
                                .map_err(|_| "invalid \\u escape".to_string())?;
                            self.pos += 4;
                            // Mirror the C++ UTF-8 encoding (BMP only).
                            if code < 0x80 {
                                out.push(code as u8 as char);
                            } else if code < 0x800 {
                                out.push(char::from_u32(0xC0 | (code >> 6)).unwrap());
                                out.push(char::from_u32(0x80 | (code & 0x3F)).unwrap());
                            } else {
                                out.push(char::from_u32(0xE0 | (code >> 12)).unwrap());
                                out.push(char::from_u32(0x80 | ((code >> 6) & 0x3F)).unwrap());
                                out.push(char::from_u32(0x80 | (code & 0x3F)).unwrap());
                            }
                        }
                        _ => return Err("invalid escape sequence".into()),
                    }
                }
                _ => {
                    // Collect raw UTF-8 bytes.
                    let start = self.pos - 1;
                    let len = utf8_len(c);
                    if start + len > self.s.len() {
                        return Err("unterminated string".into());
                    }
                    out.push_str(
                        std::str::from_utf8(&self.s[start..start + len])
                            .map_err(|_| "invalid UTF-8 in string".to_string())?,
                    );
                    self.pos = start + len;
                }
            }
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'+' | b'-') {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err("invalid number".into());
        }
        Ok(Value::Number(
            std::str::from_utf8(&self.s[start..self.pos])
                .map_err(|_| "invalid number".to_string())?
                .to_string(),
        ))
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}
