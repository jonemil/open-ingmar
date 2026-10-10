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
    let mut symbols_path = None;
    let mut data_path = None;
    let mut args = env::args_os().skip(1);
    let mut options = std::collections::BTreeSet::new();
    while let Some(arg) = args.next() {
        if arg == "--from" || arg == "--emit" || arg == "--symbols" || arg == "--data" {
            if !options.insert(arg.clone()) {
                return Err("duplicate pipeline option".into());
            }
            let value = args.next().ok_or("pipeline option requires a value")?;
            if arg == "--data" {
                data_path = Some(value);
                continue;
            }
            if arg == "--symbols" {
                symbols_path = Some(value);
                continue;
            }
            let value = value
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
        return Err("Usage: ciq-compiler [--from source|ir|assembly] [--emit ir|assembly|code] [--symbols symbol-source.mcasm] [--data data-source.mcasm] <input> <new-output>".into());
    }
    if !matches!(from.as_str(), "source" | "ir" | "assembly")
        || !matches!(emit.as_str(), "ir" | "assembly" | "code")
    {
        return Err("unsupported pipeline stage".into());
    }
    if from == "assembly" && emit == "ir" {
        return Err("assembly cannot be raised to semantic IR".into());
    }
    if (symbols_path.is_some() || data_path.is_some()) && emit != "code" {
        return Err("--symbols and --data are supported only with --emit code".into());
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
            let symbols = if let Some(path) = symbols_path {
                let symbol_source = fs::read_to_string(&path)?;
                let symbols = ciq_assembly::Program::parse(&symbol_source)
                    .map_err(|error| format!("{}: {error}", path.to_string_lossy()))?;
                ciq_compiler::symbol_bindings(&assembly, &symbols)?
            } else {
                Default::default()
            };
            let strings = if let Some(path) = data_path {
                let data_source = fs::read_to_string(&path)?;
                let data = ciq_prg::data::Source::parse_layout(&data_source)
                    .map_err(|error| format!("{}: {error}", path.to_string_lossy()))?;
                ciq_compiler::string_bindings(&assembly, &data)?
            } else {
                Default::default()
            };
            assembly.encode_with_references(&symbols, &strings)?
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
