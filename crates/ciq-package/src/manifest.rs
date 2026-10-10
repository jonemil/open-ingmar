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

/// Patch only the signing attributes of direct Garmin product elements. All
/// other lexical XML bytes, including unknown elements and comments, survive.
pub(crate) fn update_signatures(
    xml: &[u8],
    signatures: &BTreeMap<String, (String, String)>,
    limit: u64,
) -> Result<Vec<u8>> {
    let original = parse(xml)?;
    let mut reader = NsReader::from_reader(xml);
    reader.resolver_mut().set_max_declarations_per_element(128);
    let mut stack: Vec<String> = Vec::new();
    let mut edits = Vec::new();
    let mut count = 0;
    loop {
        let (resolved, event) = reader
            .read_resolved_event()
            .map_err(|e| Error::new(e.to_string()))?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let local = element.local_name();
                let garmin =
                    matches!(resolved, ResolveResult::Bound(ns) if ns.as_ref() == NAMESPACE);
                if garmin
                    && local.as_ref() == b"product"
                    && stack.as_slice() == ["manifest", "application", "products"]
                {
                    let product = original
                        .products
                        .get(count)
                        .ok_or_else(|| Error::new("product rewrite count mismatch"))?;
                    let (sha1, sha256) = signatures
                        .get(&product.filename)
                        .ok_or_else(|| Error::new("missing generated product signatures"))?;
                    let body = element.as_ref();
                    let end = usize::try_from(reader.buffer_position())
                        .map_err(|_| Error::new("XML offset overflow"))?;
                    let suffix = if matches!(event, Event::Empty(_)) {
                        3
                    } else {
                        2
                    };
                    let start = end
                        .checked_sub(body.len() + suffix)
                        .ok_or_else(|| Error::new("unsupported product XML layout"))?;
                    if xml.get(start) != Some(&b'<')
                        || xml.get(start + 1..start + 1 + body.len()) != Some(body)
                    {
                        return Err(Error::new("unsupported product XML layout"));
                    }
                    let rewritten = signing_tag(body, sha1, sha256)?;
                    edits.push((start + 1, start + 1 + body.len(), rewritten));
                    count += 1;
                }
                if matches!(event, Event::Start(_)) {
                    stack.push(if garmin {
                        std::str::from_utf8(local.as_ref())
                            .map_err(|e| Error::new(e.to_string()))?
                            .to_owned()
                    } else {
                        "<foreign>".into()
                    });
                }
            }
            Event::End(_) => {
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if count != original.products.len() {
        return Err(Error::new("product rewrite count mismatch"));
    }
    let mut result = Vec::new();
    let mut previous = 0;
    for (start, end, body) in edits {
        let growth = (start - previous)
            .checked_add(body.len())
            .ok_or_else(|| Error::new("manifest size overflow"))?;
        if (result.len() as u64).saturating_add(growth as u64) > limit {
            return Err(Error::new("generated manifest exceeds limit"));
        }
        result.extend_from_slice(&xml[previous..start]);
        result.extend_from_slice(&body);
        previous = end;
    }
    if (result.len() as u64).saturating_add((xml.len() - previous) as u64) > limit {
        return Err(Error::new("generated manifest exceeds limit"));
    }
    result.extend_from_slice(&xml[previous..]);
    let updated = parse(&result)?;
    if updated.identity != original.identity || updated.products.len() != original.products.len() {
        return Err(Error::new("product rewrite changed identity"));
    }
    Ok(result)
}
fn signing_tag(body: &[u8], sha1: &str, sha256: &str) -> Result<Vec<u8>> {
    let replacements = [
        (b"sig".as_slice(), sha1),
        (b"sig2".as_slice(), sha256),
        (b"sigFormat".as_slice(), "rsa-4096"),
    ];
    let mut found = [false; 3];
    let mut at = 0;
    while at < body.len() && !body[at].is_ascii_whitespace() {
        at += 1;
    }
    let mut result = Vec::new();
    let mut previous = 0;
    while at < body.len() {
        while at < body.len() && body[at].is_ascii_whitespace() {
            at += 1;
        }
        if at == body.len() {
            break;
        }
        let name_start = at;
        while at < body.len() && !body[at].is_ascii_whitespace() && body[at] != b'=' {
            at += 1;
        }
        let name = &body[name_start..at];
        while at < body.len() && body[at].is_ascii_whitespace() {
            at += 1;
        }
        if body.get(at) != Some(&b'=') {
            return Err(Error::new("unsupported product attribute layout"));
        }
        at += 1;
        while at < body.len() && body[at].is_ascii_whitespace() {
            at += 1;
        }
        let quote = *body
            .get(at)
            .ok_or_else(|| Error::new("missing attribute quote"))?;
        if quote != b'\'' && quote != b'"' {
            return Err(Error::new("unsupported product attribute quote"));
        }
        at += 1;
        let value_start = at;
        while at < body.len() && body[at] != quote {
            at += 1;
        }
        if at == body.len() {
            return Err(Error::new("unterminated product attribute"));
        }
        if let Some((index, (_, replacement))) = replacements
            .iter()
            .enumerate()
            .find(|(_, (key, _))| *key == name)
        {
            result.extend_from_slice(&body[previous..value_start]);
            result.extend_from_slice(replacement.as_bytes());
            previous = at;
            found[index] = true;
        }
        at += 1;
    }
    result.extend_from_slice(&body[previous..]);
    for (index, (name, value)) in replacements.iter().enumerate() {
        if !found[index] {
            result.push(b' ');
            result.extend_from_slice(name);
            result.extend_from_slice(b"=\"");
            result.extend_from_slice(value.as_bytes());
            result.push(b'"');
        }
    }
    Ok(result)
}

#[cfg(test)]
mod signing_tests {
    use super::*;
    #[test]
    fn signature_patches_preserve_unknown_xml_quotes_and_nonproduct_elements() {
        let xml = br#"<?xml version='1.0'?><!-- sig='leave' --><m:manifest xmlns:m='http://www.garmin.com/xml/connectiq' xmlns:f='urn:foreign' version='3'><m:application id='x'><m:products><m:product filename='a&amp;b.prg' partNumber='a' sig = 'old&#65;' sig2="old2" sigFormat='rsa-4096' unknown='keep &amp; exact'><m:extra/></m:product><m:product filename="a&amp;b.prg" partNumber='b' /><f:product sig='untouched'/></m:products></m:application></m:manifest>"#;
        let signatures = BTreeMap::from([("a&b.prg".into(), ("AA".into(), "BB".into()))]);
        let result = update_signatures(xml, &signatures, 4096).unwrap();
        let expected = std::str::from_utf8(xml)
            .unwrap()
            .replace("sig = 'old&#65;'", "sig = 'AA'")
            .replace("sig2=\"old2\"", "sig2=\"BB\"")
            .replace(
                "partNumber='b' /",
                "partNumber='b'  sig=\"AA\" sig2=\"BB\" sigFormat=\"rsa-4096\"/",
            );
        assert_eq!(result, expected.as_bytes());
        let parsed = parse(&result).unwrap();
        assert_eq!(parsed.products.len(), 2);
        assert!(
            parsed
                .products
                .iter()
                .all(|p| p.attributes["sig"] == "AA" && p.attributes["sig2"] == "BB")
        );
        assert!(update_signatures(xml, &signatures, 16).is_err());
        assert!(update_signatures(xml, &BTreeMap::new(), 4096).is_err());
    }
}
