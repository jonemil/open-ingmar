//! Flat scalar constant-array records and their editable DATA declarations.

use crate::symbols::source::{Error, valid_alias};
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Null,
    Number(u32),
    Boolean(bool),
    Char(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    pub alias: String,
    pub at: Option<u32>,
    pub values: Vec<Value>,
}

impl Declaration {
    /// Parse an array declaration header, retaining its optional pinned offset.
    pub fn parse_header(fields: &[String]) -> Result<Self, Error> {
        if !(fields.len() == 2 || fields.len() == 3)
            || fields[0] != ".array"
            || !valid_alias(&fields[1])
        {
            return Err(Error("expected .array <alias> [at=0x...]".into()));
        }
        let at = fields
            .get(2)
            .map(|s| {
                super::address(
                    s.strip_prefix("at=")
                        .ok_or_else(|| Error("expected array placement".into()))?,
                )
            })
            .transpose()?;
        Ok(Self {
            alias: fields[1].clone(),
            at,
            values: vec![],
        })
    }

    /// Parse and append one supported scalar element from a DATA assembly line.
    pub fn push_element(&mut self, fields: &[String]) -> Result<(), Error> {
        if fields.len() != 2 || fields[0] != ".element" {
            return Err(Error(
                "expected .element null, number=0x..., char=0x..., or boolean=true/false".into(),
            ));
        }
        let s = &fields[1];
        let value = if s == "null" {
            Value::Null
        } else if s == "boolean=true" {
            Value::Boolean(true)
        } else if s == "boolean=false" {
            Value::Boolean(false)
        } else {
            let (kind, bits) = s
                .split_once("=0x")
                .ok_or_else(|| Error("invalid scalar array element".into()))?;
            if bits.is_empty() || !bits.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error("invalid array element bit pattern".into()));
            }
            let bits = u32::from_str_radix(bits, 16)
                .map_err(|_| Error("array element exceeds u32".into()))?;
            match kind {
                "number" => Value::Number(bits),
                "char" => Value::Char(bits),
                _ => return Err(Error("unsupported scalar array element".into())),
            }
        };
        self.values.push(value);
        Ok(())
    }

    /// Calculate the complete record size, including its zero terminator.
    /// Reject lengths or element counts that cannot fit the serialized u32 fields.
    pub fn encoded_length(&self) -> Result<usize, Error> {
        u32::try_from(self.values.len()).map_err(|_| Error("array count exceeds u32".into()))?;
        let size = self.values.iter().try_fold(19usize, |n, value| {
            n.checked_add(match value {
                Value::Null => 1,
                Value::Boolean(_) => 2,
                _ => 5,
            })
            .ok_or_else(|| Error("array size overflow".into()))
        })?;
        u32::try_from(size - 6).map_err(|_| Error("array record length exceeds u32".into()))?;
        Ok(size)
    }

    /// Encode a flat scalar array, deriving both length fields and its element count.
    /// Number and Char values retain their raw 32-bit representations.
    pub fn encode(&self) -> Result<Vec<u8>, Error> {
        let size = self.encoded_length()?;
        let mut bytes = Vec::with_capacity(size);
        bytes.push(3);
        bytes.extend(((size - 6) as u32).to_be_bytes());
        bytes.extend(0xda7a_da7au32.to_be_bytes());
        bytes.extend(((size - 14) as u32).to_be_bytes());
        bytes.push(5);
        bytes.extend((self.values.len() as u32).to_be_bytes());
        for value in &self.values {
            match value {
                Value::Null => bytes.push(0),
                Value::Boolean(b) => bytes.extend([9, u8::from(*b)]),
                Value::Number(n) | Value::Char(n) => {
                    bytes.push(if matches!(value, Value::Number(_)) {
                        1
                    } else {
                        0x13
                    });
                    bytes.extend(n.to_be_bytes());
                }
            }
        }
        bytes.push(0);
        Ok(bytes)
    }

    /// Format the declaration with element types and raw Number/Char bit patterns.
    pub fn to_text(&self) -> String {
        let mut text = format!(".array {}", self.alias);
        if let Some(at) = self.at {
            write!(text, " at=0x{at:08x}").unwrap();
        }
        text.push('\n');
        for value in &self.values {
            let value = match value {
                Value::Null => "null".into(),
                Value::Boolean(b) => format!("boolean={b}"),
                Value::Number(n) => format!("number=0x{n:08x}"),
                Value::Char(n) => format!("char=0x{n:08x}"),
            };
            writeln!(text, ".element {value}").unwrap();
        }
        text.push_str(".endarray\n");
        text
    }
}

/// Parse one DATA constant-array record at a known boundary and return its end.
/// Only flat Null/Number/Boolean/Char elements are accepted. Lengths, counts,
/// magic, Boolean representations, and the terminator must agree exactly.
pub fn array_at(payload: &[u8], start: u32) -> Result<(Declaration, usize), Error> {
    let start = start as usize;
    if start >= super::ADDRESS_LIMIT {
        return Err(Error("unsupported constant-array DATA address".into()));
    }
    let header = payload
        .get(start..)
        .and_then(|s| s.get(..18))
        .ok_or_else(|| Error("truncated constant-array header".into()))?;
    if header[0] != 3 || header[5..9] != 0xda7a_da7au32.to_be_bytes() || header[13] != 5 {
        return Err(Error("unsupported constant-array framing".into()));
    }
    let length = u32::from_be_bytes(header[1..5].try_into().unwrap()) as usize;
    let inner = u32::from_be_bytes(header[9..13].try_into().unwrap()) as usize;
    if length < 13 || inner != length - 8 {
        return Err(Error("inconsistent constant-array lengths".into()));
    }
    let end = start
        .checked_add(5)
        .and_then(|s| s.checked_add(length))
        .ok_or_else(|| Error("constant-array address overflow".into()))?;
    if payload.get(end) != Some(&0) {
        return Err(Error(
            "constant-array lacks terminator or exceeds payload".into(),
        ));
    }
    let count = u32::from_be_bytes(header[14..18].try_into().unwrap()) as usize;
    let mut at = start + 18;
    if count > end - at {
        return Err(Error("array count exceeds record".into()));
    }
    let mut values = Vec::new();
    for _ in 0..count {
        let tag = *payload
            .get(at)
            .filter(|_| at < end)
            .ok_or_else(|| Error("truncated array element".into()))?;
        at += 1;
        let width = match tag {
            0 => 0,
            9 => 1,
            1 | 0x13 => 4,
            _ => return Err(Error(format!("unsupported array element tag 0x{tag:02x}"))),
        };
        if width > end - at {
            return Err(Error("truncated array element value".into()));
        }
        values.push(match tag {
            0 => Value::Null,
            9 => match payload[at] {
                0 => Value::Boolean(false),
                1 => Value::Boolean(true),
                _ => return Err(Error("unsupported serialized Boolean".into())),
            },
            1 => Value::Number(u32::from_be_bytes(payload[at..at + 4].try_into().unwrap())),
            _ => Value::Char(u32::from_be_bytes(payload[at..at + 4].try_into().unwrap())),
        });
        at += width;
    }
    if at != end {
        return Err(Error("array count does not consume record".into()));
    }
    Ok((
        Declaration {
            alias: format!("array_{start:08x}"),
            at: Some(start as u32),
            values,
        },
        end + 1,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Fragment, Source, classes};

    #[test]
    fn observed_number_record_lengths_exclude_the_terminator() {
        let bytes = [
            3, 0, 0, 0, 28, 0xda, 0x7a, 0xda, 0x7a, 0, 0, 0, 20, 5, 0, 0, 0, 3, 1, 0, 0, 0, 10, 1,
            0, 0, 0, 20, 1, 0, 0, 0, 30, 0,
        ];
        let (array, end) = array_at(&bytes, 0).unwrap();
        assert_eq!(end, bytes.len());
        assert_eq!(
            array.values,
            [Value::Number(10), Value::Number(20), Value::Number(30)]
        );
        assert_eq!(array.encode().unwrap(), bytes);
        for length in 0..bytes.len() {
            assert!(array_at(&bytes[..length], 0).is_err());
        }
        for index in [0, 4, 5, 12, 13, 17, 18, 33] {
            let mut bad = bytes;
            bad[index] = 0xff;
            assert!(array_at(&bad, 0).is_err(), "accepted corrupt byte {index}");
        }
    }

    #[test]
    fn scalar_text_roundtrip_preserves_types_bits_and_following_record_boundaries() {
        let text = ".array values\n.element null\n.element number=0xffffffff\n.element boolean=true\n.element boolean=false\n.element char=0x00000061\n.endarray\n.string after \"tail\"\n";
        let source = Source::parse(text).unwrap();
        let (bytes, strings) = source.encode().unwrap();
        assert_eq!(source.array_bindings().unwrap()["values"], 0);
        assert_eq!(strings["after"], 34);
        let records = classes::records(&bytes).unwrap();
        assert!(
            matches!(&records[0], classes::Record::Array(a) if a.values[1] == Value::Number(u32::MAX))
        );
        assert!(matches!(
            &records[1],
            classes::Record::String {
                offset: 34,
                value: "tail"
            }
        ));
        let restored = Source::from_records(&bytes).unwrap();
        assert_eq!(
            Source::parse(&restored.to_text())
                .unwrap()
                .encode()
                .unwrap()
                .0,
            bytes
        );
        let Fragment::Array(a) = &source.fragments[0] else {
            panic!()
        };
        let mut bad = a.encode().unwrap();
        bad[25] = 2;
        assert!(array_at(&bad, 0).is_err());
        for bad in [
            ".array a\n",
            ".array a\n.class x\n.endclass\n.endarray\n",
            ".array a\n.element string=0x01\n.endarray\n",
            ".array a\n.element boolean=2\n.endarray\n",
            ".array a\n.element number=0x100000000\n.endarray\n",
            ".array a\n.endarray\n.array a\n.endarray\n",
            ".array a at=0x01\n.endarray\n",
        ] {
            assert!(Source::parse(bad).is_err(), "accepted {bad}");
        }
    }
}
