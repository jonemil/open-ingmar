use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    process,
};

fn run() -> Result<(), Box<dyn Error>> {
    let mut from = "source".to_string();
    let mut emit = "assembly".to_string();
    let mut paths = Vec::new();
    let mut args = env::args_os().skip(1);
    let mut options = std::collections::BTreeSet::new();
    while let Some(arg) = args.next() {
        if arg == "--from" || arg == "--emit" {
            if !options.insert(arg.clone()) {
                return Err("duplicate pipeline option".into());
            }
            let value = args
                .next()
                .ok_or("pipeline option requires a value")?
                .into_string()
                .map_err(|_| "option value must be UTF-8")?;
            if arg == "--from" {
                from = value;
            } else {
                emit = value;
            }
        } else if arg.to_string_lossy().starts_with('-') {
            return Err(format!("unknown option {}", arg.to_string_lossy()).into());
        } else {
            paths.push(arg);
        }
    }
    if paths.len() != 2 {
        return Err("Usage: ciq-compiler [--from source|ir|assembly] [--emit ir|assembly|code] <input> <new-output>".into());
    }
    if !matches!(from.as_str(), "source" | "ir" | "assembly")
        || !matches!(emit.as_str(), "ir" | "assembly" | "code")
    {
        return Err("unsupported pipeline stage".into());
    }
    if from == "assembly" && emit == "ir" {
        return Err("assembly cannot be raised to semantic IR".into());
    }
    let source = fs::read_to_string(&paths[0])?;
    let function = match from.as_str() {
        "source" => Some(ciq_compiler::compile(&source)),
        "ir" => Some(ciq_compiler::parse_ir(&source)),
        _ => None,
    }
    .transpose()
    .map_err(|error| format!("{}:{error}", paths[0].to_string_lossy()))?;
    let bytes = if emit == "ir" {
        function.as_ref().unwrap().to_ir_text().into_bytes()
    } else {
        let assembly = if let Some(function) = function {
            function.lower()
        } else {
            ciq_assembly::Program::parse(&source)
                .map_err(|error| format!("{}: {error}", paths[0].to_string_lossy()))?
        };
        if emit == "assembly" {
            assembly.to_text().into_bytes()
        } else {
            assembly.encode()?
        }
    };
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&paths[1])?;
    output.write_all(&bytes)?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}
