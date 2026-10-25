//! Pass 2, the digital signatures of a PDF: what one proves, checked
//! without a connection.
//!
//! A PDF signature signs some bytes of the file (its `ByteRange`: all of
//! it but the hole where the signature itself goes) with a CMS envelope
//! (PKCS #7) that carries the certificates. The check:
//! - finds the `ByteRange` whose hole holds the envelope pdfium read, and
//!   whether it reaches the end of the file; what was added after it may
//!   only be long-term validation data (a `/DSS` revision);
//! - hashes those bytes and compares with the digest the envelope signed
//!   (a byte changed after signing shows here);
//! - verifies the signature with the signer's certificate (RSA PKCS #1
//!   v1.5 or PSS, or ECDSA P-256, SHA-1/256/384/512);
//! - walks the certificates of the envelope up from the signer, each
//!   verified by the next, and keeps their SHA-256 fingerprints.
//!
//! Whose seal it is, is the catalog's (R-26): a signing service is
//! recognised by the fingerprint of a certificate of the chain it pins
//! (`rootFingerprints`), and then by the names of the signer's
//! certificate. Revocation is not checked: the envelopes of the services
//! bring no OCSP, CRL or time stamp to do it without a connection.

use der::asn1::{ObjectIdentifier, OctetString};
use rusqlite::OptionalExtension;
use der::{Decode, Encode};
use rsa::pkcs1::DecodeRsaPublicKey;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha384, Sha512};
use x509_cert::spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfoOwned};
use x509_cert::Certificate;

use super::super::{sql_err, DocBase};
use super::marks::PdfSignature;
use super::types::{self, Catalog, Variant};

/// What the check of one embedded signature found.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealCheck {
    /// - `valid`: the signed bytes are the ones signed, and the signature
    ///   is the one of the signer's certificate;
    /// - `modified`: the signed bytes changed after signing;
    /// - `invalid`: the signature or its certificates do not verify, or
    ///   what it signs is not found in the file;
    /// - `unsupported`: a format or an algorithm this check does not read.
    pub state: String,
    /// Why it is not valid: `byte-range`, `envelope`, `digest`,
    /// `signature`, `certificate`, `chain`, `validity` or `algorithm`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    /// SHA-256 fingerprints (hex) of its certificates, from the signer up,
    /// each one verified by the next.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chain: Vec<String>,
    /// Common names and organizations of the signer's certificate.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signer: Vec<String>,
    /// When it was signed (RFC 3339, UTC), as the signature says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed_at: Option<String>,
    /// What of the file it signs: `file` (all of it), `file-and-validation`
    /// (all, and after it only long-term validation data) or `part`.
    #[serde(default)]
    pub covers: String,
}

impl SealCheck {
    fn failed(state: &str, problem: &str) -> Self {
        Self { state: state.to_string(), problem: Some(problem.to_string()), ..Default::default() }
    }

    pub fn is_valid(&self) -> bool {
        self.state == "valid"
    }

    /// Nothing but long-term validation data came after what it signs.
    pub fn covers_file(&self) -> bool {
        self.covers == "file" || self.covers == "file-and-validation"
    }
}

/// What the digital signatures of one file say, for the person.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealVerdict {
    /// - `sealed`: intact and sealed by a service of the catalog;
    /// - `valid-unknown`: intact, with a valid signature of someone the
    ///   catalog does not know;
    /// - `modified`: the file changed after it was signed;
    /// - `invalid`: a signature does not verify;
    /// - `unsupported`: signed in a way this check does not read.
    pub state: String,
    /// The catalog's variant of the service, and its name (the issuer).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signed_at: Option<String>,
    /// Long-term validation data was added after the signature.
    #[serde(default)]
    pub later_validation: bool,
    /// Other signatures it brings that do not verify.
    #[serde(default)]
    pub failing: usize,
    /// Of an intact file, the signatures whose envelope is not where they
    /// say: the ones of the signed documents it was put together from,
    /// which speak of each original file (P-337). Not counted in `failing`.
    #[serde(default)]
    pub joined: usize,
}

/// The signing service of the catalog whose seal this is: its chain
/// verifies up to a certificate the service pins, and the signer's
/// certificate carries one of its names. None for any other signature.
pub fn service_of<'a>(catalog: &'a Catalog, check: &SealCheck) -> Option<&'a Variant> {
    if !check.is_valid() {
        return None;
    }
    catalog.signing_services().find(|variant| {
        let pinned = variant
            .root_fingerprints
            .iter()
            .any(|pin| check.chain.iter().any(|fingerprint| fingerprint.eq_ignore_ascii_case(pin.trim())));
        let named = variant.certificate_names.is_empty()
            || variant.certificate_names.iter().any(|word| {
                let word = word.to_lowercase();
                check.signer.iter().any(|name| name.to_lowercase().contains(&word))
            });
        pinned && named
    })
}

/// What the signatures of one file say. A valid signature that covers
/// the whole file says the file is what its signer signed, whatever the
/// others say (a file put together from signed ones brings their
/// signatures, which no longer verify). Without one, the worst of them.
/// None without signatures, or when none was checked.
pub fn verdict(catalog: &Catalog, signatures: &[PdfSignature]) -> Option<SealVerdict> {
    let checks: Vec<&SealCheck> = signatures.iter().filter_map(|s| s.check.as_ref()).collect();
    if checks.is_empty() {
        return None;
    }
    let failing = checks.iter().filter(|c| c.state == "modified" || c.state == "invalid").count();
    let joined = checks.iter().filter(|c| c.state == "modified" && c.problem.as_deref() == Some("byte-range")).count();
    let plain = |state: &str| SealVerdict {
        state: state.to_string(),
        service: None,
        service_name: None,
        signed_at: None,
        later_validation: false,
        failing,
        joined: 0,
    };
    let covering: Vec<&&SealCheck> = checks.iter().filter(|c| c.is_valid() && c.covers_file()).collect();
    // The seal of a known service first.
    let chosen = covering
        .iter()
        .find(|c| service_of(catalog, c).is_some())
        .or_else(|| covering.first());
    let Some(chosen) = chosen else {
        return Some(if checks.iter().any(|c| c.state == "modified") {
            plain("modified")
        } else if checks.iter().any(|c| c.state == "invalid") {
            plain("invalid")
        } else if checks.iter().any(|c| c.is_valid()) {
            // Valid, but something not signed came after.
            plain("modified")
        } else {
            plain("unsupported")
        });
    };
    let service = service_of(catalog, chosen);
    Some(SealVerdict {
        state: if service.is_some() { "sealed" } else { "valid-unknown" }.to_string(),
        service: service.map(|v| v.id.clone()),
        service_name: service.map(|v| v.issuer.clone()).filter(|name| !name.is_empty()),
        signed_at: chosen.signed_at.clone(),
        later_validation: chosen.covers == "file-and-validation",
        failing: failing - joined,
        joined,
    })
}

impl DocBase {
    /// What the digital signatures of the file at this location say: the
    /// ones read from that file (its content may have others).
    pub fn document_seal(&self, location_id: i64) -> Result<Option<SealVerdict>, String> {
        let row: Option<(String, Option<String>)> = self
            .conn()
            .query_row(
                "SELECT l.file_hash, m.signatures FROM location l
                 JOIN file f ON f.hash = l.file_hash
                 LEFT JOIN document_marks m ON m.content_id = f.content_id
                 WHERE l.id = ?1",
                [location_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((file, Some(signatures))) = row else {
            return Ok(None);
        };
        let signatures: Vec<PdfSignature> = serde_json::from_str(&signatures).unwrap_or_default();
        let own: Vec<PdfSignature> =
            signatures.into_iter().filter(|s| s.file.as_deref().is_none_or(|from| from == file)).collect();
        Ok(verdict(types::catalog(), &own))
    }
}

// ---------------------------------------------------------------------
// The check.

/// Checks one signature of `file` whose envelope pdfium read as
/// `contents`. `pdf_time` is the signing time the PDF states (`/M`), used
/// when the envelope does not say it. `same_pages(end)` says whether the
/// file shows the same pages as its first `end` bytes (what was signed):
/// it is asked only when what came after is a validation revision whose
/// objects are compressed, so its bytes do not tell.
pub fn check(file: &[u8], contents: &[u8], pdf_time: Option<&str>, same_pages: impl Fn(usize) -> bool) -> SealCheck {
    let Some((envelope_tlv, _)) = tlv(contents) else {
        return SealCheck::failed("invalid", "envelope");
    };
    let cms = envelope_tlv.whole;
    let Some(range) = byte_ranges(file).into_iter().find(|range| hole_holds(file, range, cms)) else {
        // The envelope is not where its signature says: the file was made
        // again after signing (saved anew, joined with others).
        return SealCheck::failed("modified", "byte-range");
    };
    let [start, first, second, length] = range;
    let end = second + length;
    let after = &file[end..];
    let covers = if after.iter().all(|b| b.is_ascii_whitespace() || *b == 0) {
        "file"
    } else if validation_only(after) || (validation_compressed(after) && same_pages(end)) {
        "file-and-validation"
    } else {
        "part"
    };
    let mut out = check_envelope(cms, &[&file[start..start + first], &file[second..end]]);
    out.covers = covers.to_string();
    if out.signed_at.is_none() {
        out.signed_at = pdf_time.and_then(pdf_date);
    }
    out
}

/// The envelope `cms` against the bytes it says it signs.
pub fn check_envelope(cms: &[u8], signed: &[&[u8]]) -> SealCheck {
    let Some(envelope) = Envelope::read(cms) else {
        return SealCheck::failed("invalid", "envelope");
    };
    if !envelope.detached {
        // adbe.pkcs7.sha1: it signs a SHA-1 of the bytes, inside.
        return SealCheck::failed("unsupported", "algorithm");
    }
    let Some(signer_raw) = envelope.signers.first() else {
        return SealCheck::failed("invalid", "envelope");
    };
    let Ok(signer) = cms::signed_data::SignerInfo::from_der(signer_raw) else {
        return SealCheck::failed("invalid", "envelope");
    };
    let Some(hash) = Hash::of(&signer.digest_alg.oid) else {
        return SealCheck::failed("unsupported", "algorithm");
    };
    let content_digest = hash.digest(signed);
    let certificates: Vec<(Certificate, &[u8])> = envelope
        .certificates
        .iter()
        .filter_map(|raw| Certificate::from_der(raw).ok().map(|cert| (cert, *raw)))
        .collect();

    // What the signature is over: the signed attributes, which hold the
    // digest of the content, or the content itself.
    let mut signed_at = None;
    let attributes_signed: Vec<u8>;
    let message: Vec<&[u8]> = match &signer.signed_attrs {
        Some(attributes) => {
            let digest = attributes
                .iter()
                .find(|a| a.oid == OID_MESSAGE_DIGEST)
                .and_then(|a| a.values.get(0))
                .and_then(|value| value.to_der().ok())
                .and_then(|der| OctetString::from_der(&der).ok());
            match digest {
                Some(digest) if digest.as_bytes() == content_digest.as_slice() => {}
                Some(_) => return SealCheck::failed("modified", "digest"),
                None => return SealCheck::failed("invalid", "envelope"),
            }
            signed_at = attributes
                .iter()
                .find(|a| a.oid == OID_SIGNING_TIME)
                .and_then(|a| a.values.get(0))
                .and_then(|value| value.to_der().ok())
                .and_then(|der| x509_cert::time::Time::from_der(&der).ok())
                .and_then(|time| rfc3339(time.to_unix_duration().as_secs() as i64));
            // The signature is over the attributes as they came, as a SET.
            let Some(raw) = raw_signed_attributes(signer_raw) else {
                return SealCheck::failed("invalid", "envelope");
            };
            attributes_signed = raw;
            vec![attributes_signed.as_slice()]
        }
        None => signed.to_vec(),
    };

    let Some(signer_index) = certificates.iter().position(|(cert, _)| is_signer(cert, &signer.sid)) else {
        return SealCheck::failed("invalid", "certificate");
    };
    let signer_cert = &certificates[signer_index].0;
    match verify(
        &signer.signature_algorithm,
        Some(hash),
        &signer_cert.tbs_certificate.subject_public_key_info,
        &message,
        signer.signature.as_bytes(),
    ) {
        Verified::Yes => {}
        Verified::No => return SealCheck::failed("invalid", "signature"),
        Verified::Unknown => return SealCheck::failed("unsupported", "algorithm"),
    }

    // The chain, up from the signer.
    let mut chain = Vec::new();
    let mut current: &(Certificate, &[u8]) = &certificates[signer_index];
    let signed_secs = signed_at.as_deref().and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()).map(|t| t.timestamp());
    loop {
        if let Some(at) = signed_secs {
            let validity = &current.0.tbs_certificate.validity;
            let from = validity.not_before.to_unix_duration().as_secs() as i64;
            let to = validity.not_after.to_unix_duration().as_secs() as i64;
            if at < from || at > to {
                return SealCheck::failed("invalid", "validity");
            }
        }
        chain.push(fingerprint(current.1));
        if chain.len() > 8 {
            break;
        }
        let tbs = &current.0.tbs_certificate;
        if tbs.issuer == tbs.subject {
            // Trust comes from the catalog pinning it, not from this.
            if certificate_signed_by(current, &current.0) == Verified::No {
                return SealCheck::failed("invalid", "chain");
            }
            break;
        }
        let parents: Vec<&(Certificate, &[u8])> = certificates
            .iter()
            .filter(|(cert, _)| cert.tbs_certificate.subject == tbs.issuer && cert != &current.0)
            .collect();
        if parents.is_empty() {
            break; // the envelope does not bring the one above
        }
        let outcomes: Vec<Verified> = parents.iter().map(|parent| certificate_signed_by(current, &parent.0)).collect();
        match outcomes.iter().position(|o| *o == Verified::Yes) {
            Some(index) => current = parents[index],
            // One with the same name may be another key of the authority.
            None if outcomes.contains(&Verified::Unknown) => break, // an algorithm this check does not read
            None => return SealCheck::failed("invalid", "chain"),
        }
    }

    SealCheck {
        state: "valid".to_string(),
        problem: None,
        chain,
        signer: subject_names(signer_cert),
        signed_at,
        covers: String::new(),
    }
}

const OID_SIGNED_DATA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.7.2");
const OID_MESSAGE_DIGEST: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.4");
const OID_SIGNING_TIME: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.5");
const OID_SUBJECT_KEY_ID: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.29.14");
const OID_RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");
const OID_PSS: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.10");
const OID_MGF1: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.8");
const OID_EC_KEY: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.2.1");
const OID_P256: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.10045.3.1.7");
const OID_CN: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.3");
const OID_O: ObjectIdentifier = ObjectIdentifier::new_unwrap("2.5.4.10");

#[derive(Clone, Copy, Debug, PartialEq)]
enum Hash {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl Hash {
    fn of(oid: &ObjectIdentifier) -> Option<Self> {
        match oid.to_string().as_str() {
            "1.3.14.3.2.26" => Some(Self::Sha1),
            "2.16.840.1.101.3.4.2.1" => Some(Self::Sha256),
            "2.16.840.1.101.3.4.2.2" => Some(Self::Sha384),
            "2.16.840.1.101.3.4.2.3" => Some(Self::Sha512),
            _ => None,
        }
    }

    fn digest(self, parts: &[&[u8]]) -> Vec<u8> {
        fn run<D: Digest>(parts: &[&[u8]]) -> Vec<u8> {
            let mut hasher = D::new();
            for part in parts {
                hasher.update(part);
            }
            hasher.finalize().to_vec()
        }
        match self {
            Self::Sha1 => run::<sha1::Sha1>(parts),
            Self::Sha256 => run::<Sha256>(parts),
            Self::Sha384 => run::<Sha384>(parts),
            Self::Sha512 => run::<Sha512>(parts),
        }
    }

    fn pss(self, salt: usize) -> rsa::Pss {
        match self {
            Self::Sha1 => rsa::Pss::new_with_salt::<sha1::Sha1>(salt),
            Self::Sha256 => rsa::Pss::new_with_salt::<Sha256>(salt),
            Self::Sha384 => rsa::Pss::new_with_salt::<Sha384>(salt),
            Self::Sha512 => rsa::Pss::new_with_salt::<Sha512>(salt),
        }
    }

    fn pkcs1v15(self) -> rsa::Pkcs1v15Sign {
        match self {
            Self::Sha1 => rsa::Pkcs1v15Sign::new::<sha1::Sha1>(),
            Self::Sha256 => rsa::Pkcs1v15Sign::new::<Sha256>(),
            Self::Sha384 => rsa::Pkcs1v15Sign::new::<Sha384>(),
            Self::Sha512 => rsa::Pkcs1v15Sign::new::<Sha512>(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Verified {
    Yes,
    No,
    /// An algorithm or a key this check does not read.
    Unknown,
}

/// Verifies a signature over `message` (its parts, in order). `hash` is
/// the digest when the algorithm does not name it (`rsaEncryption`).
fn verify(algorithm: &AlgorithmIdentifierOwned, hash: Option<Hash>, key: &SubjectPublicKeyInfoOwned, message: &[&[u8]], signature: &[u8]) -> Verified {
    let mut salt = None;
    let (named_hash, family) = match algorithm.oid.to_string().as_str() {
        // RSA-PSS (RFC 4055): the hash and the salt length are its
        // parameters; the mask uses the same hash, or it is not read.
        "1.2.840.113549.1.1.10" => match pss_parameters(algorithm.parameters.as_ref()) {
            Some((pss_hash, length)) => {
                salt = Some(length);
                (Some(pss_hash), "rsa")
            }
            None => return Verified::Unknown,
        },
        "1.2.840.113549.1.1.1" => (None, "rsa"),
        "1.2.840.113549.1.1.5" => (Some(Hash::Sha1), "rsa"),
        "1.2.840.113549.1.1.11" => (Some(Hash::Sha256), "rsa"),
        "1.2.840.113549.1.1.12" => (Some(Hash::Sha384), "rsa"),
        "1.2.840.113549.1.1.13" => (Some(Hash::Sha512), "rsa"),
        "1.2.840.10045.2.1" => (None, "ecdsa"),
        "1.2.840.10045.4.1" => (Some(Hash::Sha1), "ecdsa"),
        "1.2.840.10045.4.3.2" => (Some(Hash::Sha256), "ecdsa"),
        "1.2.840.10045.4.3.3" => (Some(Hash::Sha384), "ecdsa"),
        "1.2.840.10045.4.3.4" => (Some(Hash::Sha512), "ecdsa"),
        _ => return Verified::Unknown,
    };
    let Some(hash) = named_hash.or(hash) else {
        return Verified::Unknown;
    };
    let prehashed = hash.digest(message);
    let prehashed = prehashed.as_slice();
    let Some(key_bytes) = key.subject_public_key.as_bytes() else {
        return Verified::No;
    };
    match family {
        "rsa" if key.algorithm.oid == OID_RSA || key.algorithm.oid == OID_PSS => {
            let Ok(public) = rsa::RsaPublicKey::from_pkcs1_der(key_bytes) else {
                return Verified::No;
            };
            let verified = match salt {
                Some(length) => public.verify(hash.pss(length), prehashed, signature),
                None => public.verify(hash.pkcs1v15(), prehashed, signature),
            };
            if verified.is_ok() {
                Verified::Yes
            } else {
                Verified::No
            }
        }
        "ecdsa" if key.algorithm.oid == OID_EC_KEY => {
            let curve = key.algorithm.parameters.as_ref().and_then(|p| p.decode_as::<ObjectIdentifier>().ok());
            if curve != Some(OID_P256) {
                return Verified::Unknown;
            }
            use p256::ecdsa::signature::hazmat::PrehashVerifier;
            let Ok(public) = p256::ecdsa::VerifyingKey::from_sec1_bytes(key_bytes) else {
                return Verified::No;
            };
            let Ok(signature) = p256::ecdsa::Signature::from_der(signature) else {
                return Verified::No;
            };
            if public.verify_prehash(prehashed, &signature).is_ok() {
                Verified::Yes
            } else {
                Verified::No
            }
        }
        _ => Verified::Unknown,
    }
}

/// Whether `issuer` signed `child` (a certificate and its own bytes).
fn certificate_signed_by(child: &(Certificate, &[u8]), issuer: &Certificate) -> Verified {
    let Some((outer, _)) = tlv(child.1) else {
        return Verified::No;
    };
    let Some(parts) = children(outer.value) else {
        return Verified::No;
    };
    let Some(tbs) = parts.first() else {
        return Verified::No;
    };
    let Some(signature) = child.0.signature.as_bytes() else {
        return Verified::No;
    };
    verify(&child.0.signature_algorithm, None, &issuer.tbs_certificate.subject_public_key_info, &[tbs.whole], signature)
}

/// The hash and the salt length of an RSA-PSS signature (RFC 4055), SHA-1
/// and 20 bytes when not said. None when its mask uses another hash or
/// another function, or its trailer is not the usual one.
fn pss_parameters(parameters: Option<&der::Any>) -> Option<(Hash, usize)> {
    let algorithm = |field: &Tlv<'_>| -> Option<ObjectIdentifier> {
        let first = children(field.value)?.into_iter().next()?;
        ObjectIdentifier::from_der(first.whole).ok()
    };
    let integer = |field: &Tlv<'_>| -> Option<usize> {
        (field.tag == 0x02 && field.value.len() <= 4).then(|| field.value.iter().fold(0usize, |n, b| (n << 8) | *b as usize))
    };
    let (mut hash, mut mask, mut salt) = (Hash::Sha1, Hash::Sha1, 20);
    if let Some(any) = parameters {
        let bytes = any.to_der().ok()?;
        let (outer, _) = tlv(&bytes)?;
        for field in children(outer.value)? {
            let (inner, _) = tlv(field.value)?;
            match field.tag {
                0xA0 => hash = Hash::of(&algorithm(&inner)?)?,
                0xA1 => {
                    let parts = children(inner.value)?;
                    if ObjectIdentifier::from_der(parts.first()?.whole).ok()? != OID_MGF1 {
                        return None;
                    }
                    mask = Hash::of(&algorithm(parts.get(1)?)?)?;
                }
                0xA2 => salt = integer(&inner)?,
                0xA3 if integer(&inner)? == 1 => {}
                _ => return None,
            }
        }
    }
    (hash == mask).then_some((hash, salt))
}

fn is_signer(cert: &Certificate, sid: &cms::signed_data::SignerIdentifier) -> bool {
    use cms::signed_data::SignerIdentifier;
    match sid {
        SignerIdentifier::IssuerAndSerialNumber(id) => {
            cert.tbs_certificate.issuer == id.issuer && cert.tbs_certificate.serial_number == id.serial_number
        }
        SignerIdentifier::SubjectKeyIdentifier(id) => cert
            .tbs_certificate
            .extensions
            .iter()
            .flatten()
            .filter(|ext| ext.extn_id == OID_SUBJECT_KEY_ID)
            .filter_map(|ext| x509_cert::ext::pkix::SubjectKeyIdentifier::from_der(ext.extn_value.as_bytes()).ok())
            .any(|own| own == *id),
    }
}

/// Common names and organizations of a certificate's subject.
fn subject_names(cert: &Certificate) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for rdn in cert.tbs_certificate.subject.0.iter() {
        for attribute in rdn.0.iter() {
            if attribute.oid != OID_CN && attribute.oid != OID_O {
                continue;
            }
            let value = attribute
                .value
                .decode_as::<String>()
                .ok()
                .or_else(|| attribute.value.decode_as::<der::asn1::PrintableStringRef<'_>>().ok().map(|s| s.to_string()))
                .or_else(|| attribute.value.decode_as::<der::asn1::Ia5StringRef<'_>>().ok().map(|s| s.to_string()));
            if let Some(value) = value.map(|v| v.trim().to_string()).filter(|v| !v.is_empty()) {
                if !out.contains(&value) {
                    out.push(value);
                }
            }
        }
    }
    out
}

pub fn fingerprint(der: &[u8]) -> String {
    Sha256::digest(der).iter().map(|b| format!("{b:02x}")).collect()
}

fn rfc3339(secs: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(secs, 0).map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// A PDF date (`D:20260907143000-03'00'`) in RFC 3339, UTC.
pub fn pdf_date(text: &str) -> Option<String> {
    let text = text.trim().trim_start_matches("D:");
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.len() < 8 {
        return None;
    }
    let part = |from: usize, len: usize, default: u32| -> u32 {
        digits.get(from..from + len).and_then(|s| s.parse().ok()).unwrap_or(default)
    };
    let date = chrono::NaiveDate::from_ymd_opt(part(0, 4, 0) as i32, part(4, 2, 1), part(6, 2, 1))?;
    let time = date.and_hms_opt(part(8, 2, 0), part(10, 2, 0), part(12, 2, 0))?;
    let zone = &text[digits.len()..];
    let offset_minutes = match zone.chars().next() {
        Some(sign @ ('+' | '-')) => {
            let numbers: Vec<i64> = zone[1..]
                .split(|c: char| !c.is_ascii_digit())
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse().ok())
                .collect();
            let minutes = numbers.first().copied().unwrap_or(0) * 60 + numbers.get(1).copied().unwrap_or(0);
            if sign == '-' {
                -minutes
            } else {
                minutes
            }
        }
        _ => 0,
    };
    rfc3339(time.and_utc().timestamp() - offset_minutes * 60)
}

// ---------------------------------------------------------------------
// The PDF around the envelope.

/// Every `/ByteRange [a b c d]` of the file.
pub fn byte_ranges(file: &[u8]) -> Vec<[usize; 4]> {
    const KEY: &[u8] = b"/ByteRange";
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = find(&file[from..], KEY) {
        let mut index = from + at + KEY.len();
        from = index;
        let skip = |index: &mut usize| {
            while *index < file.len() && file[*index].is_ascii_whitespace() {
                *index += 1;
            }
        };
        skip(&mut index);
        if file.get(index) != Some(&b'[') {
            continue;
        }
        index += 1;
        let mut numbers = [0usize; 4];
        let mut ok = true;
        for number in numbers.iter_mut() {
            skip(&mut index);
            let start = index;
            while index < file.len() && file[index].is_ascii_digit() {
                index += 1;
            }
            match std::str::from_utf8(&file[start..index]).ok().and_then(|s| s.parse().ok()) {
                Some(value) => *number = value,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        skip(&mut index);
        if ok && file.get(index) == Some(&b']') {
            out.push(numbers);
        }
    }
    out
}

/// The range is sound and the hole between its two parts is the hex
/// string that holds `cms`.
fn hole_holds(file: &[u8], range: &[usize; 4], cms: &[u8]) -> bool {
    let [start, first, second, length] = *range;
    let Some(hole_start) = start.checked_add(first) else {
        return false;
    };
    let Some(end) = second.checked_add(length) else {
        return false;
    };
    if start != 0 || hole_start > second || end > file.len() {
        return false;
    }
    let hole = &file[hole_start..second];
    if hole.first() != Some(&b'<') || hole.last() != Some(&b'>') {
        return false;
    }
    let mut digits = hole[1..hole.len() - 1].iter().filter(|b| !b.is_ascii_whitespace());
    for expected in cms {
        let (Some(high), Some(low)) = (digits.next(), digits.next()) else {
            return false;
        };
        let (Some(high), Some(low)) = ((*high as char).to_digit(16), (*low as char).to_digit(16)) else {
            return false;
        };
        if (high * 16 + low) as u8 != *expected {
            return false;
        }
    }
    true
}

/// What can change what a page shows, in a revision's objects.
const SEEN: &[&[u8]] = &[b"/Annots", b"/Font", b"/XObject", b"/Contents", b"/Kids", b"/Widget", b"/AP"];

/// What came after a signature is only long-term validation data: a
/// revision that adds a `/DSS` (certificates, OCSP answers, CRLs) and
/// nothing that is seen on a page.
pub fn validation_only(after: &[u8]) -> bool {
    find(after, b"/DSS").is_some()
        && find(after, b"/ObjStm").is_none()
        && SEEN.iter().all(|marker| find(after, marker).is_none())
}

/// A validation revision whose objects are compressed: whether it
/// changes a page has to be seen on the pages.
fn validation_compressed(after: &[u8]) -> bool {
    find(after, b"/DSS").is_some()
        && find(after, b"/ObjStm").is_some()
        && SEEN.iter().all(|marker| find(after, marker).is_none())
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack.windows(needle.len()).position(|window| window == needle)
}

// ---------------------------------------------------------------------
// DER, as it came (the signature is over the bytes as they came).

struct Tlv<'a> {
    tag: u8,
    value: &'a [u8],
    whole: &'a [u8],
}

/// One element and what follows it. Indefinite lengths (BER, constructed
/// elements ended by two zero bytes) are read too: some signers write
/// the envelope so; what is signed inside is DER all the same.
fn tlv(data: &[u8]) -> Option<(Tlv<'_>, &[u8])> {
    let tag = *data.first()?;
    if tag & 0x1f == 0x1f {
        return None;
    }
    let first = *data.get(1)?;
    if first == 0x80 {
        if tag & 0x20 == 0 {
            return None;
        }
        let mut rest = &data[2..];
        while !rest.starts_with(&[0, 0]) {
            rest = tlv(rest)?.1;
        }
        let end = data.len() - rest.len();
        return Some((Tlv { tag, value: &data[2..end], whole: &data[..end + 2] }, &rest[2..]));
    }
    let (len, header) = if first < 0x80 {
        (first as usize, 2)
    } else {
        let count = (first & 0x7f) as usize;
        if count == 0 || count > 4 {
            return None;
        }
        let mut len = 0usize;
        for i in 0..count {
            len = (len << 8) | *data.get(2 + i)? as usize;
        }
        (len, 2 + count)
    };
    let end = header.checked_add(len)?;
    if end > data.len() {
        return None;
    }
    Some((Tlv { tag, value: &data[header..end], whole: &data[..end] }, &data[end..]))
}

fn children(mut value: &[u8]) -> Option<Vec<Tlv<'_>>> {
    let mut out = Vec::new();
    while !value.is_empty() {
        let (item, rest) = tlv(value)?;
        out.push(item);
        value = rest;
    }
    Some(out)
}

/// The parts of a CMS SignedData, as they came.
struct Envelope<'a> {
    certificates: Vec<&'a [u8]>,
    signers: Vec<&'a [u8]>,
    /// The content is not inside (a detached signature).
    detached: bool,
}

impl<'a> Envelope<'a> {
    fn read(cms: &'a [u8]) -> Option<Self> {
        let (info, _) = tlv(cms)?;
        let parts = children(info.value)?;
        let oid = ObjectIdentifier::from_der(parts.first()?.whole).ok()?;
        if oid != OID_SIGNED_DATA || parts.get(1)?.tag != 0xa0 {
            return None;
        }
        let (signed, _) = tlv(parts[1].value)?;
        let items = children(signed.value)?;
        let encapsulated = children(items.get(2)?.value)?;
        let mut envelope = Envelope { certificates: Vec::new(), signers: Vec::new(), detached: encapsulated.len() == 1 };
        for item in items.iter().skip(3) {
            match item.tag {
                0xa0 => envelope
                    .certificates
                    .extend(children(item.value)?.into_iter().filter(|c| c.tag == 0x30).map(|c| c.whole)),
                0x31 => envelope.signers = children(item.value)?.into_iter().map(|s| s.whole).collect(),
                _ => {}
            }
        }
        Some(envelope)
    }
}

/// The signed attributes of a SignerInfo as they came, tagged as a SET.
fn raw_signed_attributes(signer: &[u8]) -> Option<Vec<u8>> {
    let (info, _) = tlv(signer)?;
    let parts = children(info.value)?;
    let attributes = parts.get(3).filter(|part| part.tag == 0xa0)?;
    let mut out = attributes.whole.to_vec();
    out[0] = 0x31;
    Some(out)
}

#[cfg(test)]
#[path = "seal_tests.rs"]
mod tests;
