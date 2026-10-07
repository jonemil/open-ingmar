//! Initial assembly grammar; see docs/specs/bytecode-subset.md.

use ciq_mc::{Instruction, Opcode};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "assembly line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for Error {}

pub fn assemble(text: &str) -> Result<Vec<u8>, Error> {
    let mut code = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let fail = |message: String| Error {
            line: line_number,
            message,
        };
        let mut fields = line.split('#').next().unwrap().split_whitespace();
        let Some(mnemonic) = fields.next() else {
            continue;
        };
        let opcode = Opcode::ALL
            .iter()
            .copied()
            .find(|opcode| opcode.mnemonic() == mnemonic)
            .ok_or_else(|| fail(format!("unsupported mnemonic {mnemonic:?}")))?;
        let values: Vec<_> = fields.collect();
        if values.len() != opcode.operand_widths().len() {
            return Err(fail(format!(
                "{} requires {} operand fields",
                mnemonic,
                opcode.operand_widths().len()
            )));
        }
        let mut operands = Vec::new();
        for (value, &width) in values.iter().zip(opcode.operand_widths()) {
            let hex = value
                .strip_prefix("0x")
                .ok_or_else(|| fail("operands must start with 0x".into()))?;
            if hex.is_empty() || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(fail(format!("invalid hexadecimal operand {value:?}")));
            }
            let number = u64::from_str_radix(hex, 16)
                .map_err(|_| fail(format!("operand {value:?} is too large")))?;
            if number >= (1u64 << (width * 8)) {
                return Err(fail(format!(
                    "operand {value:?} exceeds {width}-byte field"
                )));
            }
            operands.extend_from_slice(&number.to_be_bytes()[8 - width..]);
        }
        Instruction::new(opcode, operands)
            .map_err(|error| fail(error.to_string()))?
            .encode_into(&mut code);
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comments_and_hex_fields_preserve_bit_patterns() {
        let code = assemble("# hello\nargc 0x1\ngetmv 0x00800005 0x00800067 # System.println\ninvokem 0x02\nreturn\n").unwrap();
        assert_eq!(
            code,
            [
                0x35, 1, 0x45, 0, 0x80, 0, 5, 0, 0x80, 0, 0x67, 0x0f, 2, 0x16
            ]
        );
    }

    #[test]
    fn errors_identify_line_and_reject_invalid_operands() {
        for source in [
            "bogus",
            "return 0x1",
            "argc",
            "argc 0x100",
            "argc -1",
            "argc 1",
            "argc 0x",
            "argc 0x+1",
            "getmv 0x1",
        ] {
            let error = assemble(&format!("# comment\n{source}")).unwrap_err();
            assert_eq!(error.line, 2, "{source}");
        }
    }
}
