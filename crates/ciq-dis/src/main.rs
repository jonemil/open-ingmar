use ciq_prg::{CODE, Prg};
use std::{
    env,
    error::Error,
    fs,
    io::{self, Write},
    process,
};

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 || (args[0] != "sections" && args[0] != "code") {
        return Err("Usage: ciq-dis sections|code <input.prg>".into());
    }
    let prg = Prg::parse(&fs::read(&args[1])?)?;
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
    } else {
        stdout.write_all(ciq_dis::disassemble(prg.unique_section(CODE)?.payload())?.as_bytes())?;
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}
