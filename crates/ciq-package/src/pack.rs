use crate::{Error, Kind, Limits, Member, Package, Result, backend_error, paths};
use std::{
    fs,
    io::{self, Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};

/// Pack a canonical extracted barrel tree into a new deterministic ZIP file.
/// The source tree must remain unchanged for the duration of this operation.
pub fn pack_barrel(
    source: impl AsRef<Path>,
    output: impl AsRef<Path>,
    limits: Limits,
) -> Result<()> {
    let (source, members) = source_tree(source.as_ref(), output.as_ref(), limits)?;
    let output = output.as_ref();
    let outer = read_manifest(&source, &members, "manifest.xml", limits)?;
    let inner = read_manifest(&source, &members, "content/manifest.xml", limits)?;
    if outer != inner {
        return Err(Error::new(
            "packing requires byte-identical manifest copies",
        ));
    }
    if crate::manifest::parse(&outer)?.kind != Some(Kind::Barrel) {
        return Err(Error::new("packing requires a barrel manifest"));
    }
    if !members
        .iter()
        .any(|m| m.name == "barrel.jungle" && !m.directory)
    {
        return Err(Error::new("missing package member: barrel.jungle"));
    }
    let buffer = Bounded {
        inner: Cursor::new(Vec::new()),
        limit: limits.archive_bytes,
    };
    let mut zip = zip::ZipWriter::new(buffer);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(6))
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o644);
    for member in &members {
        // Nonempty directories are represented by their descendants. Empty ones survive.
        if member.directory {
            if !members
                .iter()
                .any(|other| other.name != member.name && other.name.starts_with(&member.name))
            {
                zip.add_directory(&member.name, options.unix_permissions(0o755))
                    .map_err(backend_error)?;
            }
        } else {
            zip.start_file(&member.name, options)
                .map_err(backend_error)?;
            let path = source.join(&member.name);
            if !fs::symlink_metadata(&path)?.file_type().is_file() {
                return Err(Error::new("source changed while packing"));
            }
            let count = io::copy(
                &mut fs::File::open(path)?.take(member.size.saturating_add(1)),
                &mut zip,
            )?;
            if count != member.size {
                return Err(Error::new("source size changed while packing"));
            }
        }
    }
    let data = zip.finish().map_err(backend_error)?.inner.into_inner();
    // Check our output against the same bounded profile accepted by the reader.
    let mut package = Package::read(data.as_slice(), limits)?;
    package
        .visit_members(|member, input| crate::copy_exact(input, &mut io::sink(), member.size))?;
    publish(output, &data)
}

fn publish(output: &Path, data: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)?;
    if let Err(error) = file.write_all(data).and_then(|()| file.flush()) {
        drop(file);
        let _ = fs::remove_file(output);
        return Err(error.into());
    }
    Ok(())
}
fn read_manifest(root: &Path, members: &[Member], name: &str, limits: Limits) -> Result<Vec<u8>> {
    let member = members
        .iter()
        .find(|m| m.name == name && !m.directory)
        .ok_or_else(|| Error::new(format!("missing package member: {name}")))?;
    if member.size > limits.manifest_bytes {
        return Err(Error::new("manifest exceeds size limit"));
    }
    let mut data = Vec::new();
    fs::File::open(root.join(name))?
        .take(limits.manifest_bytes.saturating_add(1))
        .read_to_end(&mut data)?;
    if data.len() as u64 != member.size {
        return Err(Error::new("source size changed while packing"));
    }
    Ok(data)
}
fn inventory(root: &Path, prefix: &str, members: &mut Vec<Member>, limits: Limits) -> Result<()> {
    for entry in fs::read_dir(root.join(prefix))? {
        let entry = entry?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::new("non-UTF-8 source path"))?;
        let mut name = format!("{prefix}{name}");
        let metadata = fs::symlink_metadata(entry.path())?;
        let directory = metadata.file_type().is_dir();
        if !directory && !metadata.file_type().is_file() {
            return Err(Error::new(
                "links and special source entries are unsupported",
            ));
        }
        if directory {
            name.push('/');
        }
        paths::relative(&name, directory, limits)?;
        members.push(Member {
            name: name.clone(),
            size: if directory { 0 } else { metadata.len() },
            directory,
            compression: "Deflated".into(),
        });
        if members.len() > limits.entries {
            return Err(Error::new("source entry count exceeds limit"));
        }
        if directory {
            inventory(root, &name, members, limits)?;
        }
    }
    Ok(())
}
struct Bounded {
    inner: Cursor<Vec<u8>>,
    limit: u64,
}
impl Write for Bounded {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self
            .inner
            .position()
            .checked_add(data.len() as u64)
            .is_none_or(|end| end > self.limit)
        {
            return Err(io::Error::other("archive exceeds size limit"));
        }
        self.inner.write(data)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Seek for Bounded {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let old = self.inner.position();
        let new = self.inner.seek(position)?;
        if new > self.limit {
            self.inner.set_position(old);
            return Err(io::Error::other("archive exceeds size limit"));
        }
        Ok(new)
    }
}

fn source_tree(
    source: &Path,
    output: &Path,
    limits: Limits,
) -> Result<(std::path::PathBuf, Vec<Member>)> {
    if !fs::symlink_metadata(source)?.file_type().is_dir() {
        return Err(Error::new("package source must be an ordinary directory"));
    }
    let source = fs::canonicalize(source)?;
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    if fs::canonicalize(parent)?.starts_with(&source) {
        return Err(Error::new("output must be outside the source tree"));
    }
    let mut members = Vec::new();
    inventory(&source, "", &mut members, limits)?;
    paths::validate(&members, limits)?;
    members.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((source, members))
}

/// Rebuild a modern developer IQ tree without modifying or regenerating signatures.
/// Verification must pass before a new output file is published.
pub fn repack_iq(source: impl AsRef<Path>, output: impl AsRef<Path>, limits: Limits) -> Result<()> {
    let (source, members) = source_tree(source.as_ref(), output.as_ref(), limits)?;
    let manifest = read_manifest(&source, &members, "manifest.xml", limits)?;
    if crate::manifest::parse(&manifest)?.kind != Some(Kind::Iq) {
        return Err(Error::new("repacking requires an IQ application manifest"));
    }
    let content_options = sevenz_rust2::encoder_options::EncoderOptions::Lzma2(
        sevenz_rust2::encoder_options::Lzma2Options::from_level(6),
    );
    let header_options = sevenz_rust2::encoder_options::EncoderOptions::Lzma(
        sevenz_rust2::encoder_options::LzmaOptions::default(),
    );
    if u64::from(
        content_options
            .get_lzma_dict_size()
            .max(header_options.get_lzma_dict_size()),
    ) > limits.dictionary_bytes
    {
        return Err(Error::new("IQ writer dictionary exceeds limit"));
    }
    let buffer = Bounded {
        inner: Cursor::new(Vec::new()),
        limit: limits.archive_bytes,
    };
    let mut writer = sevenz_rust2::ArchiveWriter::new(buffer).map_err(backend_error)?;
    writer.set_content_methods(vec![
        sevenz_rust2::EncoderConfiguration::new(sevenz_rust2::EncoderMethod::LZMA2)
            .with_options(content_options),
    ]);
    writer.set_encrypt_header(false);
    // One independent LZMA2 stream per ordinary file, with the pinned default options.
    for member in &members {
        if member.directory
            && members
                .iter()
                .any(|other| other.name != member.name && other.name.starts_with(&member.name))
        {
            continue;
        }
        let entry = sevenz_rust2::ArchiveEntry {
            name: member.name.trim_end_matches('/').to_owned(),
            is_directory: member.directory,
            ..Default::default()
        };
        if member.directory {
            return Err(Error::new(
                "empty directories are unsupported by the pinned IQ writer",
            ));
        } else {
            let path = source.join(&member.name);
            if !fs::symlink_metadata(&path)?.file_type().is_file() {
                return Err(Error::new("source changed while packing"));
            }
            let written = writer
                .push_archive_entry(
                    entry,
                    Some(fs::File::open(path)?.take(member.size.saturating_add(1))),
                )
                .map_err(backend_error)?;
            if written.size != member.size {
                return Err(Error::new("source size changed while packing"));
            }
        }
    }
    let data = writer.finish()?.inner.into_inner();
    let mut package = Package::read(data.as_slice(), limits)?;
    let report = package.verify(None);
    if !report.verified() {
        return Err(Error::new(format!(
            "IQ repacking requires valid modern developer signatures: {report:?}"
        )));
    }
    publish(output.as_ref(), &data)
}
