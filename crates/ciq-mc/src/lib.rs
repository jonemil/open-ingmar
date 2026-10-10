//! Instruction encodings observed in our specimens.

use std::fmt;

macro_rules! opcodes {
    ($( $variant:ident = $byte:literal, $name:literal, [$($width:literal),*]; )*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u8)]
        pub enum Opcode { $( $variant = $byte, )* }

        impl Opcode {
            pub const ALL: &'static [Self] = &[$(Self::$variant,)*];

            pub fn from_byte(byte: u8) -> Option<Self> {
                match byte { $( $byte => Some(Self::$variant), )* _ => None }
            }

            pub fn mnemonic(self) -> &'static str {
                match self { $( Self::$variant => $name, )* }
            }

            pub fn operand_widths(self) -> &'static [usize] {
                match self { $( Self::$variant => &[$($width),*], )* }
            }
        }
    };
}

opcodes! {
    Popv = 0x02, "popv", [];
    Addv = 0x03, "addv", [];
    Subv = 0x04, "subv", [];
    Divv = 0x06, "divv", [];
    Orv = 0x08, "orv", [];
    Putv = 0x0e, "putv", [];
    Invokem = 0x0f, "invokem", [1];
    Aputv = 0x11, "aputv", [];
    Lgetv = 0x12, "lgetv", [1];
    Lputv = 0x13, "lputv", [1];
    Newa = 0x14, "newa", [];
    Newc = 0x15, "newc", [];
    Return = 0x16, "return", [];
    News = 0x18, "news", [4];
    Spush = 0x27, "spush", [4];
    Frpush = 0x2a, "frpush", [];
    Dup = 0x2e, "dup", [1];
    Getm = 0x30, "getm", [];
    Argc = 0x35, "argc", [1];
    Ipushz = 0x37, "ipushz", [];
    Ipush1 = 0x38, "ipush1", [1];
    Getselfv = 0x43, "getselfv", [4];
    Getself = 0x44, "getself", [];
    Getmv = 0x45, "getmv", [4, 4];
    Getlocalv = 0x46, "getlocalv", [1, 4];
    Getsv = 0x47, "getsv", [4];
    Invokemz = 0x48, "invokemz", [];
    Argcincsp = 0x4a, "argcincsp", [1, 1];
}

impl Opcode {
    /// Zero-based operand fields that encode symbol identifiers.
    pub fn symbol_operands(self) -> &'static [usize] {
        match self {
            Self::Spush | Self::Getselfv | Self::Getsv => &[0],
            Self::Getmv => &[0, 1],
            Self::Getlocalv => &[1],
            _ => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "code offset 0x{:x}: {}", self.offset, self.message)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    opcode: Opcode,
    operands: Vec<u8>,
}

impl Instruction {
    pub fn new(opcode: Opcode, operands: Vec<u8>) -> Result<Self, Error> {
        if operands.len() != opcode.operand_widths().iter().sum::<usize>() {
            return Err(Error {
                offset: 0,
                message: "operand length does not match opcode",
            });
        }
        Ok(Self { opcode, operands })
    }

    pub fn opcode(&self) -> Opcode {
        self.opcode
    }

    pub fn operands(&self) -> &[u8] {
        &self.operands
    }

    pub fn encoded_len(&self) -> usize {
        1 + self.operands.len()
    }

    pub fn encode_into(&self, bytes: &mut Vec<u8>) {
        bytes.push(self.opcode as u8);
        bytes.extend_from_slice(&self.operands);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedInstruction {
    pub offset: usize,
    pub instruction: Instruction,
}

/// Decode the whole stream or return an error. Never skip an unknown opcode.
pub fn decode(bytes: &[u8]) -> Result<Vec<LocatedInstruction>, Error> {
    let mut result = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let opcode = Opcode::from_byte(bytes[offset]).ok_or(Error {
            offset,
            message: "unsupported opcode",
        })?;
        let operands_len = opcode.operand_widths().iter().sum::<usize>();
        if operands_len > bytes.len() - offset - 1 {
            return Err(Error {
                offset,
                message: "truncated instruction operands",
            });
        }
        let end = offset + 1 + operands_len;
        result.push(LocatedInstruction {
            offset,
            instruction: Instruction::new(opcode, bytes[offset + 1..end].to_vec())?,
        });
        offset = end;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_known_call_sequence_and_preserves_operand_bits() {
        let bytes = [
            0x35, 1, 0x45, 0, 0x80, 0, 5, 0, 0x80, 0, 0x67, 0x2a, 0x18, 0, 0, 0x32, 0x79, 0x0f, 2,
            0x02, 0x16,
        ];
        let decoded = decode(&bytes).unwrap();
        assert_eq!(
            decoded.iter().map(|entry| entry.offset).collect::<Vec<_>>(),
            [0, 2, 11, 12, 17, 19, 20]
        );
        let mut emitted = Vec::new();
        for entry in decoded {
            entry.instruction.encode_into(&mut emitted);
        }
        assert_eq!(emitted, bytes);
    }

    #[test]
    fn unknown_opcode_reports_actual_boundary() {
        let error = decode(&[0x38, 0xff, 0xff]).unwrap_err();
        assert_eq!(error.offset, 2);
        assert_eq!(error.message, "unsupported opcode");
    }

    #[test]
    fn rejects_truncated_operands_for_every_supported_opcode() {
        for &opcode in Opcode::ALL {
            let length = opcode.operand_widths().iter().sum::<usize>();
            for partial in 0..length {
                let mut bytes = vec![0x16, opcode as u8];
                bytes.extend(std::iter::repeat_n(0, partial));
                assert_eq!(decode(&bytes).unwrap_err().offset, 1);
            }
        }
        assert!(Instruction::new(Opcode::Getmv, vec![0; 4]).is_err());
    }
}
