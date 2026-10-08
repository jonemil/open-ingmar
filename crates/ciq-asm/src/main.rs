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
    const USAGE: &str = "Usage: ciq-asm <template.prg> <code.mcasm> <new-output.prg> [-y <key.der>]\n       ciq-asm project <directory> <new-output.prg> [-y <key.der>]\n       ciq-asm sign <input.prg> <new-output.prg> -y <key.der>\n       ciq-asm verify <input.prg> [-y <key.der>]\n       ciq-asm symbols|data <source.mcasm> <new-payload.bin>";
    let mut args = Vec::new();
    let mut key_path = None;
    let mut input = env::args_os().skip(1);
    while let Some(arg) = input.next() {
        if arg == "-y" {
            if key_path.is_some() {
                return Err("-y may only be supplied once".into());
            }
            key_path = Some(
                input
                    .next()
                    .ok_or("-y requires a PKCS#8 DER key filename")?,
            );
        } else {
            args.push(arg);
        }
    }
    let verifying = args.first().is_some_and(|arg| arg == "verify");
    if (verifying && args.len() != 2) || (!verifying && args.len() != 3) {
        return Err(USAGE.into());
    }
    if (args[0] == "data" || args[0] == "symbols") && key_path.is_some() {
        return Err("-y signs complete PRGs, not standalone section payloads".into());
    }
    let key = key_path
        .map(|path| -> Result<_, Box<dyn Error>> {
            Ok(ciq_sign::DeveloperKey::from_pkcs8_der(&fs::read(path)?)?)
        })
        .transpose()?;
    if verifying {
        let prg = Prg::parse(&fs::read(&args[1])?)?;
        let format = if let Some(key) = key.as_ref() {
            ciq_sign::verify_with_key(&prg, key)?
        } else {
            ciq_sign::verify(&prg)?
        };
        eprintln!(
            "Developer signature verified: {format:?}{}.",
            if key.is_some() {
                " with supplied key"
            } else {
                " against embedded public key"
            }
        );
        return Ok(());
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
    let sign_only = args[0] == "sign";
    if sign_only && key.is_none() {
        return Err("sign requires -y <key.der>".into());
    }
    let (prg, changed) = if sign_only {
        (Prg::parse(&fs::read(&args[1])?)?, false)
    } else if project {
        (ciq_asm::assemble_project(Path::new(&args[1]))?, false)
    } else {
        let original = Prg::parse(&fs::read(&args[0])?)?;
        let prg = ciq_asm::assemble_template(original.clone(), &fs::read_to_string(&args[1])?)?;
        let changed = prg != original;
        (prg, changed)
    };
    let prg = if let Some(key) = key.as_ref() {
        ciq_sign::resign(&prg, key)?
    } else {
        prg
    };
    // Refuse to overwrite the original specimen or any existing output.
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?;
    file.write_all(&prg.to_bytes())?;
    if key.is_some() {
        eprintln!(
            "Wrote {} sections with verified developer signatures from the supplied key. Simulator acceptance remains to be tested.",
            prg.sections().len()
        );
    } else if project {
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
