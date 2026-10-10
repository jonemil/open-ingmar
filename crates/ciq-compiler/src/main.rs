use std::{
    env,
    error::Error,
    fs::{self, OpenOptions},
    io::Write,
    process,
};

fn run() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: ciq-compiler <source.mc> <new-output.mcasm>".into());
    }
    let source = fs::read_to_string(&args[0])?;
    let function = ciq_compiler::compile(&source)
        .map_err(|error| format!("{}:{error}", args[0].to_string_lossy()))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[1])?;
    output.write_all(function.assembly().as_bytes())?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        process::exit(1);
    }
}
