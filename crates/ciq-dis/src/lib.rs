//! Text output for the supported ordinary bytecode subset.

use ciq_mc::{Error, decode};
use std::fmt::Write;

pub fn disassemble(code: &[u8]) -> Result<String, Error> {
    let mut text = String::new();
    // Validate the full stream before producing any output.
    for located in decode(code)? {
        let instruction = located.instruction;
        text.push_str(instruction.opcode().mnemonic());
        let mut start = 0;
        for &width in instruction.opcode().operand_widths() {
            text.push_str(" 0x");
            for byte in &instruction.operands()[start..start + width] {
                write!(text, "{byte:02x}").unwrap();
            }
            start += width;
        }
        writeln!(text, " # CODE+0x{:06x}", located.offset).unwrap();
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_follow_variable_width_instructions() {
        assert_eq!(
            disassemble(&[0x46, 2, 0, 0x80, 0, 5, 0x16]).unwrap(),
            "getlocalv 0x02 0x00800005 # CODE+0x000000\nreturn # CODE+0x000006\n"
        );
    }
}
