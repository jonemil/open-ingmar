//! PRG developer signing and verification. See docs/specs/developer-signatures.md.
use ciq_prg::{Prg, Section};
use rand_core::OsRng;
use rsa::{
    BigUint, Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey, pkcs8::DecodePrivateKey,
    traits::PublicKeyParts,
};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::fmt;

pub const DEVELOPER_SIGNATURE: u32 = 0xe1c0_de12;
pub const STORE_SIGNATURE: u32 = 0x0000_5161;
const RSA_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Sha1,
    Sha1AndSha256,
}
impl Format {
    pub fn payload_length(self) -> usize {
        match self {
            Self::Sha1 => 1028,
            Self::Sha1AndSha256 => 1540,
        }
    }
    fn from_length(length: usize) -> Result<Self, Error> {
        match length {
            1028 => Ok(Self::Sha1),
            1540 => Ok(Self::Sha1AndSha256),
            _ => Err(Error(
                "unsupported developer-signature length (expected 1028 or 1540)".into(),
            )),
        }
    }
}

/// Opaque private key; deliberately does not implement Debug or expose its components.
pub struct DeveloperKey(RsaPrivateKey);
impl DeveloperKey {
    pub fn from_pkcs8_der(bytes: &[u8]) -> Result<Self, Error> {
        let key = RsaPrivateKey::from_pkcs8_der(bytes)
            .map_err(|_| Error("expected an unencrypted PKCS#8 DER RSA private key".into()))?;
        key.validate()
            .map_err(|_| Error("invalid RSA private key".into()))?;
        validate_public_key(&RsaPublicKey::from(&key))?;
        Ok(Self(key))
    }
}
fn validate_public_key(key: &RsaPublicKey) -> Result<(), Error> {
    if key.n().bits() != RSA_BYTES * 8 {
        return Err(Error("developer signing requires an RSA-4096 key".into()));
    }
    if key.e().to_bytes_be().len() > 4 {
        return Err(Error(
            "RSA public exponent does not fit the PRG u32 field".into(),
        ));
    }
    Ok(())
}
fn signing_section(kind: u32) -> bool {
    matches!(kind, DEVELOPER_SIGNATURE | STORE_SIGNATURE)
}
/// Wrappers and payloads in original order; no signature wrappers or zero terminator.
pub fn signed_bytes(prg: &Prg) -> Vec<u8> {
    let mut bytes = Vec::new();
    for section in prg.sections().iter().filter(|s| !signing_section(s.kind())) {
        bytes.extend(section.kind().to_be_bytes());
        bytes.extend((section.payload().len() as u32).to_be_bytes());
        bytes.extend(section.payload());
    }
    bytes
}

/// Replace any original signing material with one final developer-signature section.
/// Existing supported format is retained; unsigned/store-only input defaults to dual hashes.
pub fn resign(prg: &Prg, key: &DeveloperKey) -> Result<Prg, Error> {
    let original = prg
        .sections()
        .iter()
        .filter(|s| s.kind() == DEVELOPER_SIGNATURE)
        .collect::<Vec<_>>();
    if original.len() > 1 {
        return Err(Error(
            "ambiguous duplicate developer-signature sections".into(),
        ));
    }
    let format = original
        .first()
        .map(|s| Format::from_length(s.payload().len()))
        .transpose()?
        .unwrap_or(Format::Sha1AndSha256);
    resign_with_format(prg, key, format)
}

fn resign_with_format(prg: &Prg, key: &DeveloperKey, format: Format) -> Result<Prg, Error> {
    let message = signed_bytes(prg);
    let signature = key
        .0
        .sign_with_rng(
            &mut OsRng,
            Pkcs1v15Sign::new::<Sha1>(),
            &Sha1::digest(&message),
        )
        .map_err(|_| Error("RSA SHA-1 signing failed".into()))?;
    let mut payload = Vec::with_capacity(format.payload_length());
    payload.extend(signature);
    payload.extend(key.0.n().to_bytes_be());
    let exponent = key.0.e().to_bytes_be();
    let mut exponent_word = [0; 4];
    exponent_word[4 - exponent.len()..].copy_from_slice(&exponent);
    payload.extend(exponent_word);
    if format == Format::Sha1AndSha256 {
        payload.extend(
            key.0
                .sign_with_rng(
                    &mut OsRng,
                    Pkcs1v15Sign::new::<Sha256>(),
                    &Sha256::digest(&message),
                )
                .map_err(|_| Error("RSA SHA-256 signing failed".into()))?,
        );
    }
    let mut sections = prg
        .sections()
        .iter()
        .filter(|s| !signing_section(s.kind()))
        .cloned()
        .collect::<Vec<_>>();
    sections.push(Section::new(DEVELOPER_SIGNATURE, payload).map_err(|e| Error(e.to_string()))?);
    let result = Prg::from_sections(sections);
    verify(&result)?;
    Ok(result)
}

/// Checks the embedded public key, not a trusted identity or App Store provenance.
pub fn verify(prg: &Prg) -> Result<Format, Error> {
    let sections = prg.sections();
    if sections.iter().any(|s| s.kind() == STORE_SIGNATURE) {
        return Err(Error(
            "mixed developer/store signing is unsupported for verification".into(),
        ));
    }
    let signed = prg
        .unique_section(DEVELOPER_SIGNATURE)
        .map_err(|e| Error(e.to_string()))?;
    if sections.last().map(Section::kind) != Some(DEVELOPER_SIGNATURE) {
        return Err(Error(
            "developer signature must be the final section".into(),
        ));
    }
    let payload = signed.payload();
    let format = Format::from_length(payload.len())?;
    let exponent = u32::from_be_bytes(payload[1024..1028].try_into().unwrap());
    let public = RsaPublicKey::new(
        BigUint::from_bytes_be(&payload[512..1024]),
        BigUint::from(exponent),
    )
    .map_err(|_| Error("invalid embedded RSA public key".into()))?;
    validate_public_key(&public)?;
    let message = signed_bytes(prg);
    public
        .verify(
            Pkcs1v15Sign::new::<Sha1>(),
            &Sha1::digest(&message),
            &payload[..512],
        )
        .map_err(|_| Error("developer SHA-1 signature does not verify".into()))?;
    if format == Format::Sha1AndSha256 {
        public
            .verify(
                Pkcs1v15Sign::new::<Sha256>(),
                &Sha256::digest(&message),
                &payload[1028..],
            )
            .map_err(|_| Error("developer SHA-256 signature does not verify".into()))?;
    }
    Ok(format)
}

/// Verify integrity and require the signature's public key to match the supplied key.
pub fn verify_with_key(prg: &Prg, key: &DeveloperKey) -> Result<Format, Error> {
    let format = verify(prg)?;
    let payload = prg
        .unique_section(DEVELOPER_SIGNATURE)
        .map_err(|e| Error(e.to_string()))?
        .payload();
    let exponent = u32::from_be_bytes(payload[1024..1028].try_into().unwrap());
    if key.0.n().to_bytes_be() != payload[512..1024] || *key.0.e() != BigUint::from(exponent) {
        return Err(Error(
            "embedded public key does not match the supplied developer key".into(),
        ));
    }
    Ok(format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs8::EncodePrivateKey;
    use std::sync::OnceLock;

    fn keys() -> &'static [DeveloperKey; 2] {
        static KEYS: OnceLock<[DeveloperKey; 2]> = OnceLock::new();
        KEYS.get_or_init(|| {
            std::array::from_fn(|_| {
                let private = RsaPrivateKey::new(&mut OsRng, 4096).unwrap();
                let der = private.to_pkcs8_der().unwrap();
                DeveloperKey::from_pkcs8_der(der.as_bytes()).unwrap()
            })
        })
    }
    fn specimen() -> Prg {
        Prg::from_sections(vec![
            Section::new(0x12345678, vec![9, 8, 7]).unwrap(),
            Section::new(ciq_prg::CODE, vec![0x38, 41, 0x16]).unwrap(),
            Section::new(0x12345678, vec![]).unwrap(),
        ])
    }
    fn with_signing(prg: &Prg, payload: Vec<u8>) -> Prg {
        let mut sections = prg
            .sections()
            .iter()
            .filter(|s| !signing_section(s.kind()))
            .cloned()
            .collect::<Vec<_>>();
        sections.push(Section::new(DEVELOPER_SIGNATURE, payload).unwrap());
        Prg::from_sections(sections)
    }
    #[test]
    fn coverage_includes_wrappers_order_duplicates_and_excludes_signatures_and_terminator() {
        let prg = specimen();
        let mut expected = prg.to_bytes();
        expected.truncate(expected.len() - 8);
        let mut sections = prg.sections().to_vec();
        sections.insert(1, Section::new(STORE_SIGNATURE, vec![4; 512]).unwrap());
        sections.push(Section::new(DEVELOPER_SIGNATURE, vec![3; 1540]).unwrap());
        assert_eq!(signed_bytes(&Prg::from_sections(sections)), expected);
    }
    #[test]
    fn both_formats_verify_preserve_program_and_reject_program_and_signature_tampering() {
        let key = &keys()[0];
        for format in [Format::Sha1, Format::Sha1AndSha256] {
            let input = specimen();
            let signed = resign_with_format(&input, key, format).unwrap();
            assert_eq!(verify_with_key(&signed, key).unwrap(), format);
            assert_eq!(&signed.sections()[..3], input.sections());
            assert_eq!(resign(&signed, key).unwrap(), signed); // deterministic PKCS#1 v1.5
            let mut modified = signed.clone();
            modified
                .replace_same_length(ciq_prg::CODE, &[0x38, 42, 0x16])
                .unwrap();
            assert!(verify(&modified).is_err());
            let resigned = resign(&modified, key).unwrap();
            assert_eq!(verify(&resigned).unwrap(), format);
            assert_eq!(
                resigned.unique_section(ciq_prg::CODE).unwrap().payload(),
                &[0x38, 42, 0x16]
            );
            let old = signed
                .unique_section(DEVELOPER_SIGNATURE)
                .unwrap()
                .payload();
            for at in [0, 1024, old.len() - 1] {
                let mut payload = old.to_vec();
                payload[at] ^= 1;
                assert!(verify(&with_signing(&signed, payload)).is_err());
            }
        }
    }
    #[test]
    fn another_key_replaces_signature_and_public_key_without_changing_other_sections() {
        let keys = keys();
        let first = resign(&specimen(), &keys[0]).unwrap();
        let second = resign(&first, &keys[1]).unwrap();
        assert_ne!(
            first.unique_section(DEVELOPER_SIGNATURE).unwrap().payload(),
            second
                .unique_section(DEVELOPER_SIGNATURE)
                .unwrap()
                .payload()
        );
        assert_eq!(&first.sections()[..3], &second.sections()[..3]);
        assert!(verify_with_key(&second, &keys[0]).is_err());
        assert_eq!(
            verify_with_key(&second, &keys[1]).unwrap(),
            Format::Sha1AndSha256
        );
    }
    #[test]
    fn malformed_signing_material_is_rejected_and_store_identity_is_removed_on_resign() {
        assert!(DeveloperKey::from_pkcs8_der(&[1, 2, 3]).is_err());
        assert!(
            validate_public_key(
                &RsaPublicKey::new(BigUint::from(15u8), BigUint::from(3u8)).unwrap()
            )
            .is_err()
        );
        assert!(verify(&specimen()).is_err());
        let mut store = specimen().sections().to_vec();
        store.push(Section::new(STORE_SIGNATURE, vec![0; 512]).unwrap());
        let signed = resign(&Prg::from_sections(store), &keys()[0]).unwrap();
        assert!(
            !signed
                .sections()
                .iter()
                .any(|s| s.kind() == STORE_SIGNATURE)
        );
        assert!(verify(&signed).is_ok());
        let malformed = with_signing(&specimen(), vec![0; 1027]);
        assert!(verify(&malformed).is_err());
        assert!(resign(&malformed, &keys()[0]).is_err());
        let mut duplicated = signed.sections().to_vec();
        duplicated.push(signed.sections().last().unwrap().clone());
        let duplicated = Prg::from_sections(duplicated);
        assert!(verify(&duplicated).is_err());
        assert!(resign(&duplicated, &keys()[0]).is_err());
        let mut misplaced = signed.sections().to_vec();
        misplaced.swap(0, 3);
        assert!(verify(&Prg::from_sections(misplaced)).is_err());
    }
}
