use crate::StorageError;

use super::{validate_metadata_key, validate_metadata_value};

pub(super) fn parse_privacy_metadata_json(value: &str) -> Result<(), StorageError> {
    let mut parser = JsonPolicyParser::new(value);
    parser.parse_top_object()?;
    parser.finish()
}

struct JsonPolicyParser<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> JsonPolicyParser<'a> {
    const fn new(input: &'a str) -> Self {
        Self { input, index: 0 }
    }

    fn parse_top_object(&mut self) -> Result<(), StorageError> {
        self.skip_ws();
        self.parse_object()?;
        self.skip_ws();
        Ok(())
    }

    fn finish(&self) -> Result<(), StorageError> {
        if self.index == self.input.len() {
            Ok(())
        } else {
            invalid_json()
        }
    }

    fn parse_object(&mut self) -> Result<(), StorageError> {
        self.consume(b'{')?;
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.index += 1;
            return Ok(());
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            validate_metadata_key(&key)?;
            self.skip_ws();
            self.consume(b':')?;
            self.skip_ws();
            self.parse_value()?;
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.index += 1;
                }
                Some(b'}') => {
                    self.index += 1;
                    return Ok(());
                }
                _ => return invalid_json(),
            }
        }
    }

    fn parse_array(&mut self) -> Result<(), StorageError> {
        self.consume(b'[')?;
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.index += 1;
            return Ok(());
        }
        loop {
            self.skip_ws();
            self.parse_value()?;
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.index += 1;
                }
                Some(b']') => {
                    self.index += 1;
                    return Ok(());
                }
                _ => return invalid_json(),
            }
        }
    }

    fn parse_value(&mut self) -> Result<(), StorageError> {
        match self.peek() {
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => {
                let value = self.parse_string()?;
                validate_metadata_value(&value)
            }
            Some(b't') => self.consume_literal("true"),
            Some(b'f') => self.consume_literal("false"),
            Some(b'n') => self.consume_literal("null"),
            Some(b'-' | b'0'..=b'9') => self.parse_number(),
            _ => invalid_json(),
        }
    }

    fn parse_string(&mut self) -> Result<String, StorageError> {
        self.consume(b'"')?;
        let mut output = String::new();
        while let Some(byte) = self.next_byte() {
            match byte {
                b'"' => return Ok(output),
                b'\\' => self.parse_escape(&mut output)?,
                0x00..=0x1f => return invalid_json(),
                _ => output.push(char::from(byte)),
            }
        }
        invalid_json()
    }

    fn parse_escape(&mut self, output: &mut String) -> Result<(), StorageError> {
        match self.next_byte() {
            Some(b'"') => output.push('"'),
            Some(b'\\') => output.push('\\'),
            Some(b'/') => output.push('/'),
            Some(b'b') => output.push('\u{0008}'),
            Some(b'f') => output.push('\u{000c}'),
            Some(b'n') => output.push('\n'),
            Some(b'r') => output.push('\r'),
            Some(b't') => output.push('\t'),
            Some(b'u') => output.push(self.parse_unicode_escape()?),
            _ => return invalid_json(),
        }
        Ok(())
    }

    fn parse_unicode_escape(&mut self) -> Result<char, StorageError> {
        let code_unit = self.consume_unicode_escape()?;
        match code_unit {
            0xd800..=0xdbff => {
                self.consume(b'\\')?;
                self.consume(b'u')?;
                let low = self.consume_unicode_escape()?;
                if !(0xdc00..=0xdfff).contains(&low) {
                    return invalid_json();
                }
                let scalar = 0x10000 + (((code_unit - 0xd800) << 10) | (low - 0xdc00));
                char::from_u32(scalar).ok_or_else(invalid_json_error)
            }
            0xdc00..=0xdfff => invalid_json(),
            scalar => char::from_u32(scalar).ok_or_else(invalid_json_error),
        }
    }

    fn consume_unicode_escape(&mut self) -> Result<u32, StorageError> {
        let mut value = 0;
        for _ in 0..4 {
            match self.next_byte() {
                Some(byte) => {
                    let Some(digit) = hex_digit(byte) else {
                        return invalid_json();
                    };
                    value = (value << 4) | digit;
                }
                _ => return invalid_json(),
            }
        }
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<(), StorageError> {
        if self.peek() == Some(b'-') {
            self.index += 1;
        }
        self.consume_digits()?;
        if self.peek() == Some(b'.') {
            self.index += 1;
            self.consume_digits()?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.index += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.index += 1;
            }
            self.consume_digits()?;
        }
        Ok(())
    }

    fn consume_digits(&mut self) -> Result<(), StorageError> {
        let start = self.index;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.index += 1;
        }
        if self.index > start {
            Ok(())
        } else {
            invalid_json()
        }
    }

    fn consume_literal(&mut self, literal: &str) -> Result<(), StorageError> {
        if self.input[self.index..].starts_with(literal) {
            self.index += literal.len();
            Ok(())
        } else {
            invalid_json()
        }
    }

    fn consume(&mut self, expected: u8) -> Result<(), StorageError> {
        match self.next_byte() {
            Some(byte) if byte == expected => Ok(()),
            _ => invalid_json(),
        }
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.index += 1;
        Some(byte)
    }

    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.index).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }
}

fn hex_digit(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

fn invalid_json<T>() -> Result<T, StorageError> {
    Err(invalid_json_error())
}

fn invalid_json_error() -> StorageError {
    StorageError::InvalidInput {
        field: "privacy_metadata_json",
        reason: "must be a JSON object".to_owned(),
    }
}
