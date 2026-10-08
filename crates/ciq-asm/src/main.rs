use ciq_prg::Prg;
use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    process,
};

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("Usage: ciq-asm <template.prg> <code.mcasm> <new-output.prg>\n       ciq-asm project <project-directory> <new-output.prg>\n       ciq-asm symbols|data <source.mcasm> <new-payload.bin>".into());
    }
    if args[0] == "data" || args[0] == "symbols" {
        let text = fs::read_to_string(&args[1])?;
        let bytes = if args[0] == "data" {
            ciq_prg::data::Source::parse(&text)?.encode()?.0
        } else {
            ciq_prg::symbols::source::Source::parse(&text)?.encode()?
        };
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[2])?;
        file.write_all(&bytes)?;
        eprintln!("Assembled {} payload bytes.", bytes.len());
        return Ok(());
    }
    let project = args[0] == "project";
    let (prg, changed) = if project {
        (ciq_asm::assemble_project(Path::new(&args[1]))?, false)
    } else {
        let original = Prg::parse(&fs::read(&args[0])?)?;
        let prg = ciq_asm::assemble_template(original.clone(), &fs::read_to_string(&args[1])?)?;
        let changed = prg != original;
        (prg, changed)
    };
    // Refuse to overwrite the original specimen or any existing output.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?;
    file.write_all(&prg.to_bytes())?;
    if project {
        eprintln!(
            "Reconstructed {} sections without a PRG template. Signatures were preserved, not recomputed.",
            prg.sections().len()
        );
    } else if changed {
        eprintln!(
            "Code or DATA changed. Signatures and metadata were preserved without recomputation; simulator acceptance remains to be tested."
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
