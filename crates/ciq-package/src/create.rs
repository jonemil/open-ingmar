//! Own-key creation for the supplied modern IQ metadata profile.
use crate::{Error, Kind, Limits, Result, backend_error, pack, paths};
use ciq_sign::{DeveloperKey, HashAlgorithm};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugPolicy {
    /// Omit external files named debug.xml or ending in .debug.xml.
    Omit,
    /// Preserve supplied external debug bytes; do not regenerate mappings.
    Preserve,
}
/// Create a modern IQ from supplied product metadata and PRGs, signing with our key.
pub fn pack_iq(
    source: impl AsRef<Path>,
    output: impl AsRef<Path>,
    key: &DeveloperKey,
    debug: DebugPolicy,
    limits: Limits,
) -> Result<()> {
    let output = output.as_ref();
    let (source, members) = pack::source_tree(source.as_ref(), output, limits)?;
    let xml = pack::read_manifest(&source, &members, "manifest.xml", limits)?;
    let manifest = crate::manifest::parse(&xml)?;
    if manifest.kind != Some(Kind::Iq) {
        return Err(Error::new("IQ creation requires an application manifest"));
    }
    if manifest
        .identity
        .get("apiVersion")
        .is_none_or(String::is_empty)
    {
        return Err(Error::new("IQ creation requires supplied apiVersion"));
    }
    if members.iter().any(|m| m.name == "manifest.sig") {
        return Err(Error::new(
            "legacy manifest.sig profile is unsupported for IQ creation",
        ));
    }
    let mut selected = BTreeSet::new();
    for product in &manifest.products {
        for attribute in ["connectIqVersion", "minFirmwareVersion"] {
            if product
                .attributes
                .get(attribute)
                .is_none_or(String::is_empty)
            {
                return Err(Error::new(format!(
                    "IQ creation requires supplied {attribute}"
                )));
            }
        }
        if product
            .attributes
            .get("sigFormat")
            .is_some_and(|f| f != "rsa-4096")
        {
            return Err(Error::new("unsupported IQ product signature format"));
        }
        paths::relative(&product.filename, false, limits)?;
        if matches!(
            product.filename.as_str(),
            "manifest.xml" | "manifest.sig2" | "dev_key.pub" | "manifest.sig"
        ) || (debug == DebugPolicy::Omit && is_debug(&product.filename))
        {
            return Err(Error::new(
                "product filename conflicts with generated or omitted metadata",
            ));
        }
        if !members
            .iter()
            .any(|m| m.name == product.filename && !m.directory)
        {
            return Err(Error::new(format!("missing product: {}", product.filename)));
        }
        selected.insert(product.filename.clone());
    }
    let stage = Stage::new(
        output
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new(".")),
    )?;
    let mut signatures = BTreeMap::new();
    for member in &members {
        if member.directory {
            if !members
                .iter()
                .any(|m| m.name != member.name && m.name.starts_with(&member.name))
            {
                return Err(Error::new(
                    "empty directories are unsupported by the pinned IQ writer",
                ));
            }
            continue;
        }
        if matches!(
            member.name.as_str(),
            "manifest.xml" | "manifest.sig2" | "dev_key.pub"
        ) || (debug == DebugPolicy::Omit && is_debug(&member.name))
        {
            continue;
        }
        let path = source.join(&member.name);
        if !fs::symlink_metadata(&path)?.file_type().is_file() {
            return Err(Error::new("source changed while packing"));
        }
        let mut input = fs::File::open(path)?.take(member.size.saturating_add(1));
        let destination = stage.0.join(&member.name);
        fs::create_dir_all(destination.parent().unwrap())?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination)?;
        if selected.contains(&member.name) {
            let mut bytes = Vec::new();
            input.read_to_end(&mut bytes)?;
            if bytes.len() as u64 != member.size {
                return Err(Error::new("source size changed while packing"));
            }
            let prg = ciq_prg::Prg::parse(&bytes).map_err(backend_error)?;
            if prg
                .sections()
                .iter()
                .any(|s| s.kind() == ciq_sign::STORE_SIGNATURE)
            {
                return Err(Error::new(
                    "store PRG profile is unsupported for IQ creation",
                ));
            }
            let signed = ciq_sign::resign(&prg, key).map_err(backend_error)?;
            ciq_sign::verify_with_key(&signed, key).map_err(backend_error)?;
            let fields = ciq_sign::developer_signatures(&signed).map_err(backend_error)?;
            let sha256 = fields
                .sha256
                .ok_or_else(|| Error::new("IQ creation requires dual-hash PRGs"))?;
            signatures.insert(member.name.clone(), (hex(fields.sha1), hex(sha256)));
            let bytes = signed.to_bytes();
            if bytes.len() as u64 > limits.member_bytes {
                return Err(Error::new("signed product exceeds member limit"));
            }
            file.write_all(&bytes)?;
        } else {
            crate::copy_exact(&mut input, &mut file, member.size)?;
        }
    }
    let xml = crate::manifest::update_signatures(&xml, &signatures, limits.manifest_bytes)?;
    let signature = key
        .sign_detached(HashAlgorithm::Sha256, &xml)
        .map_err(backend_error)?;
    let public = key.public_key().to_spki_der().map_err(backend_error)?;
    for (name, bytes) in [
        ("manifest.xml", xml),
        ("manifest.sig2", signature),
        ("dev_key.pub", public),
    ] {
        fs::write(stage.0.join(name), bytes)?;
    }
    // Repacking checks all output limits and validates every signature before publication.
    crate::repack_iq(&stage.0, output, limits)
}
fn is_debug(name: &str) -> bool {
    let basename = name.rsplit('/').next().unwrap_or(name);
    basename == "debug.xml" || basename.ends_with(".debug.xml")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}
struct Stage(PathBuf);
impl Stage {
    fn new(parent: &Path) -> Result<Self> {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        for _ in 0..100 {
            let path = parent.join(format!(
                ".ciq-iq-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            match crate::create_private_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(_) if path.exists() => continue,
                Err(e) => return Err(e),
            }
        }
        Err(Error::new("cannot reserve IQ staging directory"))
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
