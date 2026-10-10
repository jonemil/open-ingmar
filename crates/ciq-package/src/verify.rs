//! Verification of the documented modern developer IQ profile.
use crate::{Error, Kind, Package, Product, copy_exact};
use ciq_prg::Prg;
use ciq_sign::{DEVELOPER_SIGNATURE, Format, HashAlgorithm, PublicKey, STORE_SIGNATURE};
use std::{collections::BTreeMap, io};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    Passed,
    Failed(String),
    Unsupported(String),
    NotChecked(String),
}
impl Check {
    pub fn passed(&self) -> bool {
        matches!(self, Self::Passed)
    }
    fn result(result: Result<(), impl std::fmt::Display>) -> Self {
        match result {
            Ok(()) => Self::Passed,
            Err(e) => Self::Failed(e.to_string()),
        }
    }
}
impl std::fmt::Display for Check {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Passed => f.write_str("passed"),
            Self::Failed(message) => write!(f, "failed: {message}"),
            Self::Unsupported(message) => write!(f, "unsupported: {message}"),
            Self::NotChecked(message) => write!(f, "not checked: {message}"),
        }
    }
}
#[derive(Debug, Clone)]
pub struct ProductVerification {
    pub filename: String,
    pub part_number: String,
    pub profile: Check,
    pub prg_signatures: Check,
    pub product_signatures: Check,
    pub key_consistency: Check,
}
impl ProductVerification {
    fn pending(product: &Product) -> Self {
        let pending = Check::NotChecked("product has not been decoded".into());
        Self {
            filename: product.filename.clone(),
            part_number: product.part_number.clone(),
            profile: pending.clone(),
            prg_signatures: pending.clone(),
            product_signatures: pending.clone(),
            key_consistency: pending,
        }
    }
    pub fn verified(&self) -> bool {
        [
            &self.profile,
            &self.prg_signatures,
            &self.product_signatures,
            &self.key_consistency,
        ]
        .iter()
        .all(|c| c.passed())
    }
}
#[derive(Debug, Clone)]
pub struct VerificationReport {
    /// Decoded sizes and available member CRCs, not publisher authentication.
    pub archive_integrity: Check,
    pub public_key: Check,
    pub manifest_signature: Check,
    /// None means the caller did not request an expected signing identity.
    pub expected_key: Option<Check>,
    pub products: Vec<ProductVerification>,
    /// Ordinary files outside the documented signature coverage, e.g. debug.xml.
    pub crc_only_members: Vec<String>,
}
impl VerificationReport {
    pub fn verified(&self) -> bool {
        self.archive_integrity.passed()
            && self.public_key.passed()
            && self.manifest_signature.passed()
            && self.expected_key.as_ref().is_none_or(Check::passed)
            && !self.products.is_empty()
            && self.products.iter().all(ProductVerification::verified)
    }
}
impl Package {
    /// Check the modern developer IQ profile without writing files or reading a private key.
    pub fn verify(&mut self, expected_key: Option<&PublicKey>) -> VerificationReport {
        let pending = Check::NotChecked("no supported package signing profile".into());
        let mut report = VerificationReport {
            archive_integrity: pending.clone(),
            public_key: pending.clone(),
            manifest_signature: pending.clone(),
            expected_key: expected_key.map(|_| pending),
            products: self
                .manifest
                .products
                .iter()
                .map(ProductVerification::pending)
                .collect(),
            crc_only_members: Vec::new(),
        };
        if self.kind != Kind::Iq {
            report.manifest_signature =
                Check::Unsupported("barrels have no supported signing profile".into());
            report.archive_integrity =
                Check::result(self.visit_members(|member, reader| {
                    copy_exact(reader, &mut io::sink(), member.size)
                }));
            return report;
        }
        let public = self.read_member("dev_key.pub", 4096).and_then(|data| {
            PublicKey::from_spki_der(&data).map_err(|e| Error::new(e.to_string()))
        });
        report.public_key = match &public {
            Ok(_) => Check::Passed,
            Err(e) => Check::Failed(e.to_string()),
        };
        if let Some(expected) = expected_key {
            report.expected_key = Some(match &public {
                Ok(key) if key == expected => Check::Passed,
                Ok(_) => Check::Failed("package public key does not match the expected key".into()),
                Err(_) => Check::NotChecked("package public key is unavailable".into()),
            });
        }
        report.manifest_signature = if self.members.iter().any(|m| m.name == "manifest.sig") {
            Check::Unsupported("legacy manifest.sig signing profile is not verified".into())
        } else {
            match self.read_member("manifest.sig2", 512) {
                Ok(signature) => match &public {
                    Ok(key) => Check::result(key.verify_detached(
                        HashAlgorithm::Sha256,
                        &self.manifest_xml,
                        &signature,
                    )),
                    Err(_) => Check::NotChecked("package public key is unavailable".into()),
                },
                Err(e) => Check::Failed(e.to_string()),
            }
        };
        // Cache only small per-product reports. Process one PRG buffer at a time,
        // including duplicate descriptors which select the same ordinary member.
        let products = self.manifest.products.clone();
        let mut verified = BTreeMap::new();
        let integrity = self.visit_members(|member, reader| {
            let descriptors: Vec<_> = products
                .iter()
                .enumerate()
                .filter(|(_, p)| p.filename == member.name)
                .collect();
            if descriptors.is_empty() {
                copy_exact(reader, &mut io::sink(), member.size)?;
            } else {
                let mut bytes = Vec::new();
                copy_exact(reader, &mut bytes, member.size)?;
                let prg = Prg::parse(&bytes);
                for (index, product) in descriptors {
                    let product_report = match &prg {
                        Ok(prg) => verify_product(product, prg, public.as_ref().ok()),
                        Err(e) => {
                            let mut result = ProductVerification::pending(product);
                            result.prg_signatures = Check::Failed(format!("invalid PRG: {e}"));
                            result
                        }
                    };
                    verified.insert(index, product_report);
                }
            }
            Ok(())
        });
        report.archive_integrity = Check::result(integrity);
        for (index, product) in verified {
            report.products[index] = product;
        }
        report.crc_only_members = self
            .members
            .iter()
            .filter(|m| {
                !m.directory
                    && !matches!(
                        m.name.as_str(),
                        "manifest.xml" | "manifest.sig2" | "dev_key.pub"
                    )
                    && !products.iter().any(|p| p.filename == m.name)
            })
            .map(|m| m.name.clone())
            .collect();
        report
    }
}
fn verify_product(
    product: &Product,
    prg: &Prg,
    package_key: Option<&PublicKey>,
) -> ProductVerification {
    let mut report = ProductVerification::pending(product);
    report.profile = match product.attributes.get("sigFormat").map(String::as_str) {
        Some("rsa-4096") => Check::Passed,
        other => Check::Unsupported(format!("product sigFormat {other:?}")),
    };
    if prg.sections().iter().any(|s| s.kind() == STORE_SIGNATURE) {
        report.prg_signatures = Check::Unsupported("store signing profile".into());
        return report;
    }
    if let Ok(section) = prg.unique_section(DEVELOPER_SIGNATURE) {
        if !matches!(section.payload().len(), 1028 | 1540) {
            report.prg_signatures =
                Check::Unsupported("unknown developer-signature payload".into());
            return report;
        }
    }
    let fields = match ciq_sign::developer_signatures(prg) {
        Ok(fields) => fields,
        Err(e) => {
            report.prg_signatures = Check::Failed(e.to_string());
            return report;
        }
    };
    report.prg_signatures = Check::result(ciq_sign::verify(prg).map(|_| ()));
    report.key_consistency = match package_key {
        Some(key) if *key == fields.public_key => Check::Passed,
        Some(_) => Check::Failed("PRG embedded key differs from package public key".into()),
        None => Check::NotChecked("package public key is unavailable".into()),
    };
    if fields.format != Format::Sha1AndSha256 {
        report.profile =
            Check::Unsupported("IQ products require the observed dual-hash PRG profile".into());
        return report;
    }
    if report.profile.passed() {
        report.product_signatures = Check::result((|| {
            let sha1 = signature_attribute(product, "sig")?;
            let sha256 = signature_attribute(product, "sig2")?;
            if sha1 != fields.sha1 || Some(sha256.as_slice()) != fields.sha256 {
                return Err(Error::new("product attributes differ from PRG signatures"));
            }
            Ok(())
        })());
    } else {
        report.product_signatures =
            Check::NotChecked("unsupported product signature format".into());
    }
    report
}
fn signature_attribute(product: &Product, name: &str) -> Result<Vec<u8>, Error> {
    let text = product
        .attributes
        .get(name)
        .ok_or_else(|| Error::new(format!("missing product {name}")))?;
    if text.len() != 1024 {
        return Err(Error::new(format!(
            "product {name} must contain 1024 hexadecimal characters"
        )));
    }
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            fn digit(n: u8) -> Option<u8> {
                match n {
                    b'0'..=b'9' => Some(n - b'0'),
                    b'a'..=b'f' => Some(n - b'a' + 10),
                    b'A'..=b'F' => Some(n - b'A' + 10),
                    _ => None,
                }
            }
            let high =
                digit(pair[0]).ok_or_else(|| Error::new("invalid signature hexadecimal digit"))?;
            let low =
                digit(pair[1]).ok_or_else(|| Error::new("invalid signature hexadecimal digit"))?;
            Ok(high * 16 + low)
        })
        .collect()
}
