use ciq_prg::{CODE, Prg};
use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    process,
};

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("Usage: ciq-asm <template.prg> <code.mcasm> <output.prg>".into());
    }
    let mut prg = Prg::parse(&fs::read(&args[0])?)?;
    let code = ciq_asm::assemble(&fs::read_to_string(&args[1])?)?;
    let changed = prg.unique_section(CODE)?.payload() != code;
    prg.replace_same_length(CODE, &code)?;
    // Refuse to overwrite the original specimen or any existing output.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?;
    file.write_all(&prg.to_bytes())?;
    if changed {
        eprintln!(
            "Code changed. Signatures and metadata were preserved without recomputation; simulator acceptance remains to be tested."
        );
    } else {
        eprintln!("Reassembled code is unchanged; PRG sections were preserved.");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}
