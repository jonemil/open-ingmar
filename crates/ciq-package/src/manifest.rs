use crate::{Error, Kind, Result};
use quick_xml::{XmlVersion, events::Event, name::ResolveResult, reader::NsReader};
use std::collections::BTreeMap;
const NAMESPACE: &[u8] = b"http://www.garmin.com/xml/connectiq";
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Manifest {
    pub kind: Option<Kind>,
    /// Attributes on the application or barrel element, including unknown attributes.
    pub identity: BTreeMap<String, String>,
    pub products: Vec<Product>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Product {
    pub filename: String,
    pub part_number: String,
    pub attributes: BTreeMap<String, String>,
}
pub(crate) fn parse(xml: &[u8]) -> Result<Manifest> {
    std::str::from_utf8(xml).map_err(|e| Error::new(e.to_string()))?;
    let mut reader = NsReader::from_reader(xml);
    reader.resolver_mut().set_max_declarations_per_element(128);
    let mut declaration = false;
    let mut stack: Vec<String> = Vec::new();
    let mut root = false;
    let mut ended = false;
    let mut manifest = Manifest::default();
    loop {
        let (resolved, event) = reader
            .read_resolved_event()
            .map_err(|e| Error::new(e.to_string()))?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                if ended || stack.len() >= 128 {
                    return Err(Error::new("invalid manifest nesting"));
                }
                let local = element.local_name();
                let local = std::str::from_utf8(local.as_ref())
                    .map_err(|e| Error::new(e.to_string()))?
                    .to_owned();
                let garmin =
                    matches!(resolved, ResolveResult::Bound(ns) if ns.as_ref() == NAMESPACE);
                let mut attrs = BTreeMap::new();
                for attr in element.attributes() {
                    let attr = attr.map_err(|e| Error::new(e.to_string()))?;
                    if attrs.len() >= 128 {
                        return Err(Error::new("too many manifest attributes"));
                    }
                    let key = std::str::from_utf8(attr.key.as_ref())
                        .map_err(|e| Error::new(e.to_string()))?
                        .to_owned();
                    let value = attr
                        .decoded_and_normalized_value(XmlVersion::Implicit1_0, reader.decoder())
                        .map_err(|e| Error::new(e.to_string()))?
                        .into_owned();
                    attrs.insert(key, value);
                }
                if stack.is_empty() {
                    if root
                        || !garmin
                        || local != "manifest"
                        || attrs.get("version").map(String::as_str) != Some("3")
                    {
                        return Err(Error::new("expected Connect IQ version 3 manifest"));
                    }
                    root = true;
                } else if stack.len() == 1
                    && garmin
                    && matches!(local.as_str(), "barrel" | "application")
                {
                    if manifest.kind.is_some() {
                        return Err(Error::new("multiple package identities"));
                    }
                    let kind = if local == "barrel" {
                        Kind::Barrel
                    } else {
                        Kind::Iq
                    };
                    for key in if kind == Kind::Barrel {
                        &["id", "module", "version"][..]
                    } else {
                        &["id"][..]
                    } {
                        if attrs.get(*key).is_none_or(String::is_empty) {
                            return Err(Error::new(format!("missing manifest attribute: {key}")));
                        }
                    }
                    manifest.kind = Some(kind);
                    manifest.identity = attrs;
                } else if stack.as_slice() == ["manifest", "application", "products"]
                    && garmin
                    && local == "product"
                {
                    let filename = attrs
                        .get("filename")
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| Error::new("missing IQ product filename"))?
                        .clone();
                    let part_number = attrs
                        .get("partNumber")
                        .filter(|s| !s.is_empty())
                        .ok_or_else(|| Error::new("missing IQ product partNumber"))?
                        .clone();
                    manifest.products.push(Product {
                        filename,
                        part_number,
                        attributes: attrs,
                    });
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(if garmin { local } else { "<foreign>".into() });
                } else if stack.is_empty() {
                    ended = true;
                }
            }
            Event::End(_) => {
                stack.pop();
                if stack.is_empty() {
                    ended = true;
                }
            }
            Event::DocType(_) => return Err(Error::new("manifest DTDs are unsupported")),
            Event::Text(text) => {
                if stack.is_empty() && text.as_ref().iter().any(|b| !b.is_ascii_whitespace()) {
                    return Err(Error::new("text outside manifest root"));
                }
            }
            Event::CData(_) if stack.is_empty() => {
                return Err(Error::new("CDATA outside manifest root"));
            }
            Event::Decl(decl) => {
                if root
                    || declaration
                    || decl
                        .version()
                        .map_err(|e| Error::new(e.to_string()))?
                        .as_ref()
                        != b"1.0"
                {
                    return Err(Error::new(
                        "expected a single XML 1.0 declaration before the root",
                    ));
                }
                declaration = true;
                if let Some(encoding) = decl.encoding() {
                    let encoding = encoding.map_err(|e| Error::new(e.to_string()))?;
                    if !encoding.eq_ignore_ascii_case(b"utf-8")
                        && !encoding.eq_ignore_ascii_case(b"us-ascii")
                    {
                        return Err(Error::new("unsupported manifest encoding"));
                    }
                }
            }
            Event::GeneralRef(reference) => {
                if stack.is_empty() {
                    return Err(Error::new("entity reference outside manifest root"));
                }
                let numeric = reference
                    .resolve_char_ref()
                    .map_err(|e| Error::new(e.to_string()))?;
                let name = reference.decode().map_err(|e| Error::new(e.to_string()))?;
                if numeric.is_none()
                    && quick_xml::escape::resolve_predefined_entity(&name).is_none()
                {
                    return Err(Error::new("undefined manifest entity"));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !root || !ended || !stack.is_empty() || manifest.kind.is_none() {
        return Err(Error::new("incomplete package manifest"));
    }
    if manifest.kind == Some(Kind::Iq) && manifest.products.is_empty() {
        return Err(Error::new("IQ manifest has no products"));
    }
    Ok(manifest)
}
