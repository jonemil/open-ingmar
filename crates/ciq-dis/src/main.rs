use ciq_prg::{Prg, SYMBOLS, symbols::Symbols};
use std::{
    env,
    error::Error,
    fs,
    io::{self, Write},
    path::Path,
    process,
};

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let inventory = args.len() == 2
        && (args[0] == "sections"
            || args[0] == "code"
            || args[0] == "symbols"
            || args[0] == "symbol-source"
            || args[0] == "code-symbolic"
            || args[0] == "strings"
            || args[0] == "data-source"
            || args[0] == "classes"
            || args[0] == "resource-strings");
    let export = args.len() == 3 && args[0] == "all";
    if !inventory && !export {
        return Err("Usage: ciq-dis sections|code|code-symbolic|symbols|symbol-source|strings|data-source|resource-strings|classes <input.prg>\n       ciq-dis all <input.prg> <new-project-directory>".into());
    }
    let prg = Prg::parse(&fs::read(&args[1])?)?;
    if export {
        let manifest = ciq_dis::export_project(&prg, Path::new(&args[2]))?;
        println!(
            "Exported {} sections to {}",
            manifest.entries.len(),
            Path::new(&args[2]).display()
        );
        return Ok(());
    }
    let mut stdout = io::stdout().lock();
    if args[0] == "sections" {
        let mut offset = 0;
        for section in prg.sections() {
            writeln!(
                stdout,
                "{offset:08x}  {:08x}  {:8} bytes  {}",
                section.kind(),
                section.payload().len(),
                section.name().unwrap_or("UNKNOWN")
            )?;
            offset += 8 + section.payload().len();
        }
        writeln!(stdout, "{offset:08x}  00000000  terminator")?;
    } else if args[0] == "resource-strings" {
        stdout.write_all(ciq_dis::resource_strings(&prg)?.as_bytes())?;
    } else if args[0] == "classes" {
        stdout.write_all(ciq_dis::classes(&prg)?.as_bytes())?;
    } else if args[0] == "strings" {
        stdout.write_all(ciq_dis::strings(&prg)?.as_bytes())?;
    } else if args[0] == "data-source" {
        stdout.write_all(ciq_dis::data_source(&prg)?.as_bytes())?;
    } else if args[0] == "symbol-source" {
        stdout.write_all(ciq_dis::symbol_source(&prg)?.as_bytes())?;
    } else if args[0] == "code-symbolic" {
        stdout.write_all(ciq_dis::disassemble_symbolic(&prg)?.as_bytes())?;
    } else if args[0] == "symbols" {
        let symbols = Symbols::parse(prg.unique_section(SYMBOLS)?.payload())?;
        stdout.write_all(ciq_dis::format_symbols(&symbols).as_bytes())?;
    } else {
        stdout.write_all(ciq_dis::disassemble_prg(&prg)?.as_bytes())?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}
