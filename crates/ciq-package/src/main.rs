use ciq_package::{Limits, Package};
use std::{env, path::PathBuf};
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() < 2
        || (args[0] != "list" && args[0] != "unpack")
        || (args[0] == "list" && args.len() != 2)
        || (args[0] == "unpack" && args.len() != 3)
    {
        return Err("usage: ciq-package list <archive> | unpack <archive> <new-directory>".into());
    }
    let mut package = Package::open(PathBuf::from(&args[1]), Limits::default())?;
    if args[0] == "unpack" {
        package.unpack(PathBuf::from(&args[2]))?;
        println!(
            "Extracted {} members to {}. Decoded sizes and available member CRCs checked.",
            package.members().len(),
            PathBuf::from(&args[2]).display()
        );
    } else {
        println!("{:?}: {:?}", package.kind(), package.manifest().identity);
        for product in &package.manifest().products {
            println!("Product {:?}: {}", product.part_number, product.filename);
        }
        for member in package.members() {
            println!(
                "{} {:>10}  {:<12} {}",
                if member.directory { "d" } else { "f" },
                member.size,
                member.compression,
                member.name
            );
        }
        println!("Inventory only; payload CRCs have not all been checked.");
    }
    println!("Signatures were not verified.");
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("ciq-package: {e}");
        std::process::exit(1);
    }
}
