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
    let mut unit = false;
    let mut symbols_path = None;
    let mut data_path = None;
    let mut app_id = None;
    let mut app_entry = None;
    let mut target_api = None;
    let mut min_api = None;
    let mut args = env::args_os().skip(1);
    let mut options = std::collections::BTreeSet::new();
    while let Some(arg) = args.next() {
        if arg == "--unit" {
            if unit {
                return Err("duplicate --unit option".into());
            }
            unit = true;
            continue;
        }
        if arg == "--from"
            || arg == "--emit"
            || arg == "--symbols"
            || arg == "--data"
            || arg == "--app-id"
            || arg == "--entry"
            || arg == "--target-api"
            || arg == "--min-api"
        {
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
            if arg == "--app-id" {
                app_id = Some(value);
            } else if arg == "--entry" {
                app_entry = Some(value);
            } else if arg == "--target-api" {
                target_api = Some(value.parse::<ciq_compiler::api::ApiLevel>()?);
            } else if arg == "--min-api" {
                min_api = Some(value.parse::<ciq_compiler::api::ApiLevel>()?);
            } else if arg == "--from" {
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
    let context = match (target_api, min_api) {
        (None, None) => None,
        (Some(target), Some(minimum)) => Some(ciq_compiler::api::ApiContext::new(target, minimum)?),
        _ => return Err("--target-api and --min-api must be supplied together".into()),
    };
    if context.is_some() && from == "assembly" {
        return Err("API checks require source or IR input; assembly has no API contracts".into());
    }
    if paths.len() < 2 || (paths.len() != 2 && (!unit || from != "source")) {
        return Err("Usage: ciq-compiler [--unit] [--from source|ir|assembly] [--emit ir|assembly|code|prg] [--target-api <major.minor.patch> --min-api <major.minor.patch>] [--app-id <32-hex-digits>] [--entry <global-function|Class.method>] [--symbols symbol-source.mcasm] [--data data-source.mcasm] <input> [additional-unit-source ...] <new-output>".into());
    }
    if !matches!(from.as_str(), "source" | "ir" | "assembly")
        || !matches!(emit.as_str(), "ir" | "assembly" | "code" | "prg")
    {
        return Err("unsupported pipeline stage".into());
    }
    if from == "assembly" && emit == "ir" {
        return Err("assembly cannot be raised to semantic IR".into());
    }
    if (symbols_path.is_some() || data_path.is_some()) && emit != "code" {
        return Err("--symbols and --data are supported only with --emit code".into());
    }
    if emit == "prg" && (!unit || from == "assembly") {
        return Err("--emit prg requires --unit with source or unit IR input".into());
    }
    if emit != "prg" && (app_id.is_some() || app_entry.is_some()) {
        return Err("--app-id and --entry are supported only with --emit prg".into());
    }
    let application = if emit == "prg" {
        let text = app_id.ok_or("--emit prg requires --app-id <32-hex-digits>")?;
        if text.len() != 32 || !text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("application ID requires exactly 32 hexadecimal digits".into());
        }
        let mut id = [0; 16];
        for (index, byte) in id.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)?;
        }
        Some(ciq_compiler::application::ConsoleApplication {
            id,
            entry: app_entry.unwrap_or_else(|| "main".into()),
        })
    } else {
        None
    };
    let mut prg = None;
    let output_path = paths.pop().unwrap();
    let sources = paths
        .iter()
        .map(fs::read_to_string)
        .collect::<Result<Vec<_>, _>>()?;
    let source = &sources[0];
    let (ir, assembly) = if unit && from != "assembly" {
        let model = if from == "source" {
            let inputs = sources.iter().map(String::as_str).collect::<Vec<_>>();
            let result = if let Some(context) = context {
                ciq_compiler::compile_sources_with_api(&inputs, context)
            } else {
                ciq_compiler::compile_sources(&inputs)
            };
            result.map_err(|error| {
                format!(
                    "{}:{}",
                    paths[error.source].to_string_lossy(),
                    error.diagnostic
                )
            })?
        } else {
            let result = if let Some(context) = context {
                ciq_compiler::parse_unit_ir_with_api(source, context)
            } else {
                ciq_compiler::parse_unit_ir(source)
            };
            result.map_err(|error| format!("{}:{error}", paths[0].to_string_lossy()))?
        };
        if let Some(application) = &application {
            prg = Some(
                ciq_compiler::application::build_console_application(&model, application)?
                    .to_bytes(),
            );
            (None, None)
        } else {
            (
                (emit == "ir").then(|| model.to_ir_text()),
                (emit != "ir").then(|| model.lower()),
            )
        }
    } else if from != "assembly" {
        let function = match (from.as_str(), context) {
            ("source", Some(context)) => ciq_compiler::compile_with_api(source, context),
            ("source", None) => ciq_compiler::compile(source),
            (_, Some(context)) => ciq_compiler::parse_ir_with_api(source, context),
            (_, None) => ciq_compiler::parse_ir(source),
        }
        .map_err(|error| format!("{}:{error}", paths[0].to_string_lossy()))?;
        (
            (emit == "ir").then(|| function.to_ir_text()),
            (emit != "ir").then(|| function.lower()),
        )
    } else {
        (
            None,
            Some(
                ciq_assembly::Program::parse(source)
                    .map_err(|error| format!("{}: {error}", paths[0].to_string_lossy()))?,
            ),
        )
    };
    let bytes = if emit == "prg" {
        prg.unwrap()
    } else if emit == "ir" {
        ir.unwrap().into_bytes()
    } else {
        let assembly = assembly.unwrap();
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
            let (strings, arrays) = if let Some(path) = data_path {
                let data_source = fs::read_to_string(&path)?;
                let data = ciq_prg::data::Source::parse_layout(&data_source)
                    .map_err(|error| format!("{}: {error}", path.to_string_lossy()))?;
                (
                    ciq_compiler::string_bindings(&assembly, &data)?,
                    ciq_compiler::array_bindings(&assembly, &data)?,
                )
            } else {
                Default::default()
            };
            assembly.encode_with_literals(&symbols, &strings, &arrays)?
        }
    };
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output_path)?;
    output.write_all(&bytes)?;
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}
