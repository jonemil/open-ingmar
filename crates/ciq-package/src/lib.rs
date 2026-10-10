//! Package reading, verification and barrel packing; see docs/specs/package-reading.md
//! and docs/specs/barrel-packing.md.
mod manifest;
mod pack;
pub use pack::pack_barrel;
mod paths;
mod preflight;
mod verify;
pub use verify::{Check, ProductVerification, VerificationReport};

pub use manifest::{Manifest, Product};
use std::{
    fmt, fs,
    io::{self, Cursor, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub struct Error(String);
impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::new(e.to_string())
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub entries: usize,
    pub archive_bytes: u64,
    pub header_bytes: usize,
    pub member_bytes: u64,
    pub total_bytes: u64,
    pub manifest_bytes: u64,
    pub dictionary_bytes: u64,
    pub path_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            entries: 4096,
            archive_bytes: 64 << 20,
            header_bytes: 4 << 20,
            member_bytes: 128 << 20,
            total_bytes: 1 << 30,
            manifest_bytes: 2 << 20,
            dictionary_bytes: 64 << 20,
            path_bytes: 1024,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Barrel,
    Iq,
}
#[derive(Debug, Clone)]
pub struct Member {
    pub name: String,
    pub size: u64,
    pub directory: bool,
    pub compression: String,
}
enum Backend {
    Zip(Box<zip::ZipArchive<Cursor<Vec<u8>>>>),
    SevenZ(Box<sevenz_rust2::ArchiveReader<Cursor<Vec<u8>>>>),
}
pub struct Package {
    backend: Backend,
    kind: Kind,
    members: Vec<Member>,
    manifest: Manifest,
    /// Exact outer manifest bytes; never reserialized during extraction.
    manifest_xml: Vec<u8>,
    limits: Limits,
}
fn backend_error(e: impl fmt::Display) -> Error {
    Error::new(e.to_string())
}
fn check_mode(mode: u32) -> Result<()> {
    match mode & 0o170000 {
        0 | 0o100000 | 0o040000 => Ok(()),
        _ => Err(Error::new(
            "links and special archive entries are unsupported",
        )),
    }
}
impl Package {
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub fn members(&self) -> &[Member] {
        &self.members
    }
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }
    pub fn manifest_xml(&self) -> &[u8] {
        &self.manifest_xml
    }
    pub fn open(path: impl AsRef<Path>, limits: Limits) -> Result<Self> {
        Self::read(fs::File::open(path)?, limits)
    }
    pub fn read(mut input: impl Read, limits: Limits) -> Result<Self> {
        let mut data = Vec::new();
        (&mut input)
            .take(limits.archive_bytes.saturating_add(1))
            .read_to_end(&mut data)?;
        if data.len() as u64 > limits.archive_bytes {
            return Err(Error::new("archive exceeds size limit"));
        }
        let (backend, kind, members) = if data.starts_with(b"PK\x03\x04") {
            let count = preflight::zip(&data, limits)?;
            let mut archive = zip::ZipArchive::new(Cursor::new(data)).map_err(backend_error)?;
            if archive.len() != count {
                return Err(Error::new("duplicate ZIP entries"));
            }
            let mut members = Vec::new();
            for i in 0..archive.len() {
                let entry = archive.by_index_raw(i).map_err(backend_error)?;
                if entry.encrypted() {
                    return Err(Error::new("encrypted ZIP is unsupported"));
                }
                check_mode(entry.unix_mode().unwrap_or(0))?;
                if !matches!(
                    entry.compression(),
                    zip::CompressionMethod::Stored | zip::CompressionMethod::Deflated
                ) {
                    return Err(Error::new("unsupported ZIP compression"));
                }
                let name = std::str::from_utf8(entry.name_raw())
                    .map_err(backend_error)?
                    .to_owned();
                members.push(Member {
                    name,
                    size: entry.size(),
                    directory: entry.is_dir(),
                    compression: format!("{:?}", entry.compression()),
                });
            }
            (Backend::Zip(Box::new(archive)), Kind::Barrel, members)
        } else if data.starts_with(b"7z\xbc\xaf\x27\x1c") {
            preflight::seven_z(&data, limits)?;
            let mut archive = sevenz_rust2::ArchiveReader::new(
                Cursor::new(data),
                sevenz_rust2::Password::empty(),
            )
            .map_err(backend_error)?;
            archive.set_thread_count(1);
            let mut members = Vec::new();
            for (index, entry) in archive.archive().files.iter().enumerate() {
                if entry.size == 0 && entry.has_crc && entry.crc != 0 {
                    return Err(Error::new("invalid empty-member CRC"));
                }
                if entry.is_anti_item
                    || (entry.has_windows_attributes && entry.windows_attributes & 0x400 != 0)
                {
                    return Err(Error::new(
                        "7z anti-items and reparse points are unsupported",
                    ));
                }
                if entry.has_windows_attributes {
                    check_mode(entry.windows_attributes >> 16)?;
                }
                members.push(Member {
                    name: entry.name.clone(),
                    size: entry.size,
                    directory: entry.is_directory,
                    compression: match archive.archive().stream_map.file_block_index[index] {
                        Some(block) => {
                            match archive.archive().blocks[block].coders[0].encoder_method_id() {
                                [0] => "Copy",
                                [0x21] => "LZMA2",
                                [3, 1, 1] => "LZMA",
                                _ => "unsupported",
                            }
                        }
                        None => "empty",
                    }
                    .into(),
                });
            }
            (Backend::SevenZ(Box::new(archive)), Kind::Iq, members)
        } else {
            return Err(Error::new("expected a barrel ZIP or IQ 7z archive"));
        };
        paths::validate(&members, limits)?;
        let mut package = Self {
            backend,
            kind,
            members,
            manifest: Manifest::default(),
            manifest_xml: Vec::new(),
            limits,
        };
        let raw = package.read_manifest("manifest.xml")?;
        let manifest = manifest::parse(&raw)?;
        if manifest.kind != Some(kind) {
            return Err(Error::new("manifest kind does not match package container"));
        }
        if kind == Kind::Barrel {
            let inner = manifest::parse(&package.read_manifest("content/manifest.xml")?)?;
            if inner.kind != Some(kind)
                || ["id", "module", "version"]
                    .iter()
                    .any(|key| inner.identity.get(*key) != manifest.identity.get(*key))
            {
                return Err(Error::new("barrel manifests have different identities"));
            }
            package.require_file("barrel.jungle")?;
        } else {
            for product in &manifest.products {
                paths::relative(&product.filename, false, limits)?;
                package.require_file(&product.filename)?;
            }
        }
        package.manifest = manifest;
        package.manifest_xml = raw;
        Ok(package)
    }
    fn require_file(&self, name: &str) -> Result<&Member> {
        self.members
            .iter()
            .find(|m| m.name == name && !m.directory)
            .ok_or_else(|| Error::new(format!("missing package member: {name}")))
    }
    fn read_manifest(&mut self, name: &str) -> Result<Vec<u8>> {
        self.read_member(name, self.limits.manifest_bytes)
    }
    pub(crate) fn read_member(&mut self, name: &str, limit: u64) -> Result<Vec<u8>> {
        let size = self.require_file(name)?.size;
        if size > limit {
            return Err(Error::new("requested member exceeds size limit"));
        }
        let mut output = Vec::new();
        match &mut self.backend {
            Backend::Zip(archive) => {
                let mut file = archive.by_name(name).map_err(backend_error)?;
                copy_exact(&mut file, &mut output, size)?;
            }
            Backend::SevenZ(archive) => {
                let mut found = false;
                archive
                    .for_each_entries(|entry, reader| {
                        if found {
                            return Ok(false);
                        }
                        let result = if entry.name == name {
                            found = true;
                            copy_exact(reader, &mut output, entry.size)
                        } else {
                            copy_exact(reader, &mut io::sink(), entry.size)
                        };
                        result.map_err(|e| sevenz_rust2::Error::Other(e.to_string().into()))?;
                        Ok(!found)
                    })
                    .map_err(backend_error)?;
                if !found {
                    return Err(Error::new("manifest not decoded"));
                }
            }
        }
        Ok(output)
    }
    /// Extract into a new directory. Payloads are checked before publishing files.
    /// Existing destinations are always refused, including empty directories.
    pub fn unpack(&mut self, destination: impl AsRef<Path>) -> Result<()> {
        let destination = destination.as_ref();
        create_private_dir(destination)?;
        let result = self.unpack_reserved(destination);
        if result.is_err() {
            let _ = fs::remove_dir_all(destination);
        }
        result
    }
    fn unpack_reserved(&mut self, destination: &Path) -> Result<()> {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let mut stage = None;
        for _ in 0..100 {
            let path = parent.join(format!(
                ".ciq-package-{}-{}",
                std::process::id(),
                SERIAL.fetch_add(1, Ordering::Relaxed)
            ));
            match create_private_dir(&path) {
                Ok(()) => {
                    stage = Some(path);
                    break;
                }
                Err(e) if path.exists() => {
                    let _ = e;
                }
                Err(e) => return Err(e),
            }
        }
        let stage =
            stage.ok_or_else(|| Error::new("cannot reserve extraction staging directory"))?;
        let result = self.extract_to(&stage).and_then(|()| {
            for entry in fs::read_dir(&stage)? {
                let entry = entry?;
                fs::rename(entry.path(), destination.join(entry.file_name()))?;
            }
            Ok(())
        });
        let cleanup = fs::remove_dir_all(&stage);
        result?;
        cleanup?;
        Ok(())
    }
    fn extract_to(&mut self, stage: &Path) -> Result<()> {
        self.visit_members(|member, reader| extract_member(stage, member, reader))
    }
    /// Internal streaming adapter: callbacks must consume the complete member.
    fn visit_members(
        &mut self,
        mut visitor: impl FnMut(&Member, &mut dyn Read) -> Result<()>,
    ) -> Result<()> {
        match &mut self.backend {
            Backend::Zip(archive) => {
                for (i, member) in self.members.iter().enumerate() {
                    let mut reader = archive.by_index(i).map_err(backend_error)?;
                    visitor(member, &mut reader)?;
                }
            }
            Backend::SevenZ(archive) => {
                let members = &self.members;
                let mut visited = std::collections::BTreeSet::new();
                archive
                    .for_each_entries(|entry, reader| {
                        let member =
                            members
                                .iter()
                                .find(|m| m.name == entry.name)
                                .ok_or_else(|| {
                                    sevenz_rust2::Error::Other("unexpected decoded member".into())
                                })?;
                        if !visited.insert(entry.name.clone()) {
                            return Err(sevenz_rust2::Error::Other(
                                "duplicate decoded member".into(),
                            ));
                        }
                        visitor(member, reader)
                            .map_err(|e| sevenz_rust2::Error::Other(e.to_string().into()))?;
                        Ok(true)
                    })
                    .map_err(backend_error)?;
                if visited.len() != members.len() {
                    return Err(Error::new("not all package members were decoded"));
                }
            }
        }
        Ok(())
    }
}
fn create_private_dir(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)?;
    Ok(())
}
fn extract_member(stage: &Path, member: &Member, reader: &mut dyn Read) -> Result<()> {
    let path: PathBuf = stage.join(member.name.trim_end_matches('/'));
    if member.directory {
        fs::create_dir_all(path)?;
        copy_exact(reader, &mut io::sink(), 0)?;
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        copy_exact(reader, &mut file, member.size)?;
    }
    Ok(())
}
fn copy_exact(reader: &mut dyn Read, writer: &mut dyn Write, expected: u64) -> Result<()> {
    let mut remaining = expected;
    let mut buffer = [0u8; 65536];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        if count as u64 > remaining {
            return Err(Error::new("decoded member exceeds declared size"));
        }
        writer.write_all(&buffer[..count])?;
        remaining -= count as u64;
    }
    if remaining != 0 {
        return Err(Error::new("truncated decoded member"));
    }
    Ok(())
}
