use ciq_package::{Limits, Package};
use std::{env, path::PathBuf};
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() < 2
        || (args[0] != "list" && args[0] != "unpack" && args[0] != "verify")
        || (args[0] == "list" && args.len() != 2)
        || (args[0] == "unpack" && args.len() != 3)
        || (args[0] == "verify"
            && !(args.len() == 2 || (args.len() == 4 && args[2] == "--expected-key")))
    {
        return Err("usage: ciq-package list <archive> | unpack <archive> <new-directory> | verify <archive> [--expected-key <public.der>]".into());
    }
    let mut package = Package::open(PathBuf::from(&args[1]), Limits::default())?;
    if args[0] == "verify" {
        let expected = if args.len() == 4 {
            use std::io::Read;
            let mut bytes = Vec::new();
            std::fs::File::open(&args[3])?
                .take(4097)
                .read_to_end(&mut bytes)?;
            Some(ciq_sign::PublicKey::from_spki_der(&bytes)?)
        } else {
            None
        };
        let report = package.verify(expected.as_ref());
        println!(
            "Archive decoded sizes / available CRCs: {}",
            report.archive_integrity
        );
        println!("Package public key: {}", report.public_key);
        println!("Manifest SHA-256 signature: {}", report.manifest_signature);
        if let Some(check) = &report.expected_key {
            println!("Expected public key: {check}");
        }
        for product in &report.products {
            println!("Product {:?}: {}", product.part_number, product.filename);
            for (name, check) in [
                ("Profile", &product.profile),
                ("PRG signatures", &product.prg_signatures),
                ("Product signature attributes", &product.product_signatures),
                ("Package/PRG key consistency", &product.key_consistency),
            ] {
                println!("  {name}: {check}");
            }
        }
        if !report.crc_only_members.is_empty() {
            println!(
                "Outside signature coverage (CRC checks only): {:?}",
                report.crc_only_members
            );
        }
        println!(
            "Verification: {}",
            if report.verified() {
                "passed (modern developer IQ profile)"
            } else {
                "failed or unsupported"
            }
        );
        println!(
            "Publisher identity requires an independently obtained expected key. Store acceptance is not checked."
        );
        if !report.verified() {
            return Err("package verification did not pass all required checks".into());
        }
        return Ok(());
    }
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
