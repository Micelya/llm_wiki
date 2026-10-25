//! The check of embedded signatures, with keys, certificates and a
//! signed PDF invented here.

use std::str::FromStr;
use std::time::Duration;

use base64::Engine;
use cms::cert::{CertificateChoices, IssuerAndSerialNumber};
use cms::content_info::{CmsVersion, ContentInfo};
use cms::signed_data::{CertificateSet, EncapsulatedContentInfo, SignedData, SignerIdentifier, SignerInfo, SignerInfos};
use der::asn1::{Any, OctetString, SetOfVec, UtcTime};
use der::Encode;
use p256::ecdsa::signature::{Keypair, Signer};
use rsa::pkcs1::DecodeRsaPrivateKey;
use x509_cert::attr::Attribute;
use x509_cert::builder::{Builder, CertificateBuilder, Profile};
use x509_cert::name::Name;
use x509_cert::serial_number::SerialNumber;
use x509_cert::spki::{AlgorithmIdentifierOwned, SubjectPublicKeyInfoOwned};
use x509_cert::time::Validity;
use x509_cert::Certificate;

use super::*;
use crate::docbase::similar::marks::PdfSignature;

/// An invented RSA key of 1024 bits (PKCS #1), for the root.
const ROOT_KEY: &str = "MIICXAIBAAKBgQDL1PTV4YUpwZPeP9/AdoeGm3KJpFKvdK+GCVc92jR1u2H6dxnVNUYCO8K3rxn28vCmuMU+AI7t7IVeU5Fg1ZAz4aXLWUfsqLerq5HYUWSil/HxC4QyuI4t15mZRz/GKxYuj+21GdyO9N9NCuqQLX2gjOGSjfRIPYbee67z++LFbwIDAQABAoGBAIjR5pKWscMInFAIAhJ3vZr+RxDbS2FY6uUG6herm6EXabotLhruipK/OIIBuGUKR70xs7xdtZNBO3jIkYI1eHgYKNTWO+DoCh7yKrhhwDCfw1QFLi45ZuM3lmpMCuKb8gjQWrH5JqUINiGPX9Xm4SsBh2Pm5CMc0TX0RS+Pv+l5AkEA5LUd2z2jS1B25bKfgploDu1ly1Kq0I4vXHA1bzyPYWfXMEbQN6jD0vZC5z9PqsyVxp68Gm6YVTmm3uKk9ysPgwJBAOQn51Aw5bVl0L34oEeAdA9btTAhfwRFE/sdRlZASlSLggh/gDcV/c11aPaLEbrw83s00ngbjyl+gjbLmYS2QqUCQFTaabBc5wfbW7GPGyU+O2BGbd0I170285VtQDX8FwpBl5Iew8PSxoyvt1TKpYkuDGe7nviDjdsY37OUDTZy8HcCQEz3GRT4KcVTn2M2Z/s23fAAY9LkudBa7S6c5YlbDrgOh8MOH9Z1WLTikpH4v/ZsKfa6DvXtdhHtV0fczuDyCvECQBwxPFQQCNNCPLIqa7yP2QEnYG8hJU3bR6VRD4D23gGHap8ruxvqxhxPbhIa/qChAjnTC/zeo/7J5//LZ48qwIc=";

const DATA: &str = "1.2.840.113549.1.7.1";

struct Issuer {
    key: rsa::pkcs1v15::SigningKey<Sha256>,
    cert: Certificate,
}

fn root() -> Issuer {
    let der = base64::engine::general_purpose::STANDARD.decode(ROOT_KEY).unwrap();
    let private = rsa::RsaPrivateKey::from_pkcs1_der(&der).unwrap();
    let key = rsa::pkcs1v15::SigningKey::<Sha256>::new(private);
    let spki = SubjectPublicKeyInfoOwned::from_key(key.verifying_key()).unwrap();
    let cert = CertificateBuilder::new(
        Profile::Root,
        SerialNumber::from(1u32),
        Validity::from_now(Duration::from_secs(3650 * 86400)).unwrap(),
        Name::from_str("CN=Servicio de prueba Raiz,O=Servicio de prueba").unwrap(),
        spki,
        &key,
    )
    .unwrap()
    .build::<rsa::pkcs1v15::Signature>()
    .unwrap();
    Issuer { key, cert }
}

fn leaf_key(seed: u8) -> p256::ecdsa::SigningKey {
    p256::ecdsa::SigningKey::from_bytes(&[seed; 32].into()).unwrap()
}

/// The service's seal certificate, issued by the root.
fn leaf(root: &Issuer, key: &p256::ecdsa::SigningKey, subject: &str) -> Certificate {
    let spki = SubjectPublicKeyInfoOwned::from_key(*key.verifying_key()).unwrap();
    CertificateBuilder::new(
        Profile::Leaf { issuer: root.cert.tbs_certificate.subject.clone(), enable_key_agreement: false, enable_key_encipherment: false },
        SerialNumber::from(2u32),
        Validity::from_now(Duration::from_secs(365 * 86400)).unwrap(),
        Name::from_str(subject).unwrap(),
        spki,
        &root.key,
    )
    .unwrap()
    .build::<rsa::pkcs1v15::Signature>()
    .unwrap()
}

/// A certificate that signs itself, with the service's name.
fn self_signed(key: &p256::ecdsa::SigningKey, subject: &str) -> Certificate {
    let spki = SubjectPublicKeyInfoOwned::from_key(*key.verifying_key()).unwrap();
    CertificateBuilder::new(
        Profile::Root,
        SerialNumber::from(3u32),
        Validity::from_now(Duration::from_secs(365 * 86400)).unwrap(),
        Name::from_str(subject).unwrap(),
        spki,
        key,
    )
    .unwrap()
    .build::<p256::ecdsa::DerSignature>()
    .unwrap()
}

fn attribute(oid: &str, value: Any) -> Attribute {
    Attribute { oid: ObjectIdentifier::new_unwrap(oid), values: SetOfVec::try_from(vec![value]).unwrap() }
}

/// A detached CMS envelope over `digest`, signed by `key` (the owner of
/// `signer`), carrying `certificates`.
fn envelope(digest: &[u8], key: &p256::ecdsa::SigningKey, signer: &Certificate, certificates: &[&Certificate]) -> Vec<u8> {
    let sha256 = AlgorithmIdentifierOwned { oid: ObjectIdentifier::new_unwrap("2.16.840.1.101.3.4.2.1"), parameters: None };
    let now = UtcTime::from_unix_duration(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap()).unwrap();
    let attributes: SetOfVec<Attribute> = SetOfVec::try_from(vec![
        attribute("1.2.840.113549.1.9.3", Any::encode_from(&ObjectIdentifier::new_unwrap(DATA)).unwrap()),
        attribute("1.2.840.113549.1.9.4", Any::encode_from(&OctetString::new(digest.to_vec()).unwrap()).unwrap()),
        attribute("1.2.840.113549.1.9.5", Any::encode_from(&now).unwrap()),
    ])
    .unwrap();
    let signature: p256::ecdsa::DerSignature = key.sign(&attributes.to_der().unwrap());
    let signer_info = SignerInfo {
        version: CmsVersion::V1,
        sid: SignerIdentifier::IssuerAndSerialNumber(IssuerAndSerialNumber {
            issuer: signer.tbs_certificate.issuer.clone(),
            serial_number: signer.tbs_certificate.serial_number.clone(),
        }),
        digest_alg: sha256.clone(),
        signed_attrs: Some(attributes),
        signature_algorithm: AlgorithmIdentifierOwned { oid: ObjectIdentifier::new_unwrap("1.2.840.10045.4.3.2"), parameters: None },
        signature: OctetString::new(signature.as_bytes().to_vec()).unwrap(),
        unsigned_attrs: None,
    };
    let signed = SignedData {
        version: CmsVersion::V1,
        digest_algorithms: SetOfVec::try_from(vec![sha256]).unwrap(),
        encap_content_info: EncapsulatedContentInfo { econtent_type: ObjectIdentifier::new_unwrap(DATA), econtent: None },
        certificates: Some(CertificateSet(
            SetOfVec::try_from(certificates.iter().map(|c| CertificateChoices::Certificate((*c).clone())).collect::<Vec<_>>()).unwrap(),
        )),
        crls: None,
        signer_infos: SignerInfos(SetOfVec::try_from(vec![signer_info]).unwrap()),
    };
    ContentInfo { content_type: OID_SIGNED_DATA, content: Any::encode_from(&signed).unwrap() }.to_der().unwrap()
}

const HOLE: usize = 8192;

/// A minimal PDF whose signature dictionary signs all of it but the
/// hole, and the hole's contents as pdfium gives them (with the zeros).
fn signed_pdf(sign: impl Fn(&[u8]) -> Vec<u8>) -> (Vec<u8>, Vec<u8>) {
    let head = "%PDF-1.7\n1 0 obj\n<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange [";
    let blank = " ".repeat(40);
    let middle = "] /Contents <";
    let tail = "> >>\nendobj\n2 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 2 0 R >>\n%%EOF\n";
    let hole_start = head.len() + blank.len() + middle.len() - 1;
    let hole_end = hole_start + 1 + HOLE * 2 + 1;
    let total = hole_end + tail.len() - 1;
    let numbers = format!("0 {hole_start} {hole_end} {}", total - hole_end);
    let mut file = format!("{head}{numbers:<40}{middle}").into_bytes();
    file.extend(std::iter::repeat(b'0').take(HOLE * 2));
    file.extend(tail.as_bytes());
    assert_eq!(file.len(), total);
    assert_eq!(file[hole_start], b'<');
    assert_eq!(file[hole_end - 1], b'>');
    let digest = Sha256::new().chain_update(&file[..hole_start]).chain_update(&file[hole_end..]).finalize();
    let cms = sign(&digest);
    let hex: String = cms.iter().map(|b| format!("{b:02X}")).collect();
    file[hole_start + 1..hole_start + 1 + hex.len()].copy_from_slice(hex.as_bytes());
    let mut contents = cms;
    contents.resize(HOLE, 0);
    (file, contents)
}

const SUBJECT: &str = "CN=Sello de prueba,O=Servicio de prueba";

fn catalog_pinning(fingerprint: &str) -> Catalog {
    Catalog::parse(&format!(
        r#"{{"version": 1, "types": [{{"id": "pdf-servicio-de-firma", "carrier": true, "variants": [
            {{"id": "servicio-de-prueba", "issuer": "Servicio de prueba", "certificateNames": ["Servicio de prueba"],
              "rootFingerprints": ["{fingerprint}"]}}]}}]}}"#
    ))
    .unwrap()
}

fn sealed_by(root: &Issuer) -> (Vec<u8>, Vec<u8>) {
    let key = leaf_key(7);
    let cert = leaf(root, &key, SUBJECT);
    signed_pdf(|digest| envelope(digest, &key, &cert, &[&cert, &root.cert]))
}

fn with_check(check: SealCheck) -> Vec<PdfSignature> {
    vec![PdfSignature { check: Some(check), ..Default::default() }]
}

#[test]
fn a_valid_seal_of_a_known_service_is_intact_and_sealed() {
    let root = root();
    let (file, contents) = sealed_by(&root);
    let found = check(&file, &contents, None, |_| false);
    assert_eq!((found.state.as_str(), found.covers.as_str(), found.problem.as_deref()), ("valid", "file", None));
    let root_fingerprint = fingerprint(&root.cert.to_der().unwrap());
    assert_eq!(found.chain.len(), 2);
    assert_eq!(found.chain[1], root_fingerprint);
    assert!(found.signer.contains(&"Servicio de prueba".to_string()));
    assert!(found.signed_at.is_some());

    let catalog = catalog_pinning(&root_fingerprint);
    assert_eq!(service_of(&catalog, &found).map(|v| v.id.as_str()), Some("servicio-de-prueba"));
    let verdict = verdict(&catalog, &with_check(found)).unwrap();
    assert_eq!(verdict.state, "sealed");
    assert_eq!(verdict.service_name.as_deref(), Some("Servicio de prueba"));
    assert!(!verdict.later_validation);
}

#[test]
fn a_byte_changed_after_signing_is_a_modified_file() {
    let (mut file, contents) = sealed_by(&root());
    file[7] = b'6'; // %PDF-1.7 → %PDF-1.6
    let found = check(&file, &contents, None, |_| false);
    assert_eq!((found.state.as_str(), found.problem.as_deref()), ("modified", Some("digest")));
    let catalog = catalog_pinning("00");
    assert_eq!(verdict(&catalog, &with_check(found)).unwrap().state, "modified");
}

#[test]
fn a_signature_that_is_not_the_signers_does_not_verify() {
    let root = root();
    let key = leaf_key(7);
    let cert = leaf(&root, &key, SUBJECT);
    // Signed with another key than the certificate's.
    let (file, contents) = signed_pdf(|digest| envelope(digest, &leaf_key(9), &cert, &[&cert, &root.cert]));
    let found = check(&file, &contents, None, |_| false);
    assert_eq!((found.state.as_str(), found.problem.as_deref()), ("invalid", Some("signature")));
    assert_eq!(verdict(&catalog_pinning("00"), &with_check(found)).unwrap().state, "invalid");
}

#[test]
fn a_root_the_catalog_does_not_pin_is_a_valid_signature_of_someone_unknown() {
    let root = root();
    let (file, contents) = sealed_by(&root);
    let found = check(&file, &contents, None, |_| false);
    assert!(found.is_valid());
    let other = catalog_pinning(&"ab".repeat(32));
    assert!(service_of(&other, &found).is_none());
    assert_eq!(verdict(&other, &with_check(found)).unwrap().state, "valid-unknown");

    // A certificate that signs itself with the service's name is not its seal.
    let key = leaf_key(5);
    let fake = self_signed(&key, SUBJECT);
    let (file, contents) = signed_pdf(|digest| envelope(digest, &key, &fake, &[&fake]));
    let found = check(&file, &contents, None, |_| false);
    assert!(found.is_valid());
    assert!(found.signer.contains(&"Servicio de prueba".to_string()));
    let pinned = catalog_pinning(&fingerprint(&root.cert.to_der().unwrap()));
    assert!(service_of(&pinned, &found).is_none());
}

#[test]
fn what_is_added_after_signing_is_only_long_term_validation() {
    let root = root();
    let pinned = catalog_pinning(&fingerprint(&root.cert.to_der().unwrap()));
    let (file, contents) = sealed_by(&root);

    let mut validation = file.clone();
    validation.extend(b"3 0 obj\n<< /Type /Catalog /DSS 4 0 R >>\nendobj\n4 0 obj\n<< /Certs [5 0 R] >>\nendobj\n5 0 obj\n<< /Length 3 >>\nstream\nabc\nendstream\nendobj\ntrailer\n<< /Root 3 0 R >>\n%%EOF\n");
    let found = check(&validation, &contents, None, |_| false);
    assert_eq!((found.state.as_str(), found.covers.as_str()), ("valid", "file-and-validation"));
    let verdict_with_validation = verdict(&pinned, &with_check(found)).unwrap();
    assert_eq!(verdict_with_validation.state, "sealed");
    assert!(verdict_with_validation.later_validation);

    let mut changed = file.clone();
    changed.extend(b"3 0 obj\n<< /Type /Annot /Subtype /FreeText /Contents (otro texto) >>\nendobj\n%%EOF\n");
    let found = check(&changed, &contents, None, |_| false);
    assert_eq!((found.state.as_str(), found.covers.as_str()), ("valid", "part"));
    assert_eq!(verdict(&pinned, &with_check(found)).unwrap().state, "modified");
}

#[test]
fn an_envelope_that_is_not_the_one_in_the_file_does_not_verify() {
    let (file, mut contents) = sealed_by(&root());
    contents[100] ^= 0xff;
    let found = check(&file, &contents, None, |_| false);
    assert_eq!((found.state.as_str(), found.problem.as_deref()), ("modified", Some("byte-range")));
}

#[test]
fn byte_ranges_and_pdf_dates_are_read() {
    assert_eq!(byte_ranges(b"<< /ByteRange [0 10 20 5] >> /ByteRange[ 0  3\n9 1 ]"), vec![[0, 10, 20, 5], [0, 3, 9, 1]]);
    assert!(byte_ranges(b"/ByteRange [0 10 20]").is_empty());
    assert_eq!(pdf_date("D:20260907143000-03'00'").as_deref(), Some("2026-09-07T17:30:00Z"));
    assert_eq!(pdf_date("D:20260907143000Z").as_deref(), Some("2026-09-07T14:30:00Z"));
    assert_eq!(pdf_date("D:20260907").as_deref(), Some("2026-09-07T00:00:00Z"));
    assert_eq!(pdf_date("ayer"), None);
}

#[test]
fn the_signatures_of_the_documents_a_file_was_put_together_from_are_told_apart() {
    // A file put together from signed documents and then signed whole:
    // the envelopes of the documents are not where their signatures say.
    let whole = SealCheck { state: "valid".into(), covers: "file".into(), ..Default::default() };
    let signatures: Vec<PdfSignature> = [whole, SealCheck::failed("modified", "byte-range"), SealCheck::failed("modified", "byte-range"), SealCheck::failed("invalid", "signature")]
        .into_iter()
        .map(|check| PdfSignature { check: Some(check), ..Default::default() })
        .collect();
    let found = verdict(&catalog_pinning("00"), &signatures).unwrap();
    assert_eq!((found.state.as_str(), found.joined, found.failing), ("valid-unknown", 2, 1));
    // Without a signature of the whole file, it changed after signing.
    let found = verdict(&catalog_pinning("00"), &signatures[1..]).unwrap();
    assert_eq!((found.state.as_str(), found.joined, found.failing), ("modified", 0, 3));
}

/// A stand-in random source for the RSA test key and its salt.
struct Counter(u64);

impl rsa::rand_core::RngCore for Counter {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 ^ (self.0 >> 33)
    }
    fn fill_bytes(&mut self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(8) {
            let value = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&value[..chunk.len()]);
        }
    }
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rsa::rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl rsa::rand_core::CryptoRng for Counter {}

#[test]
fn an_rsa_pss_signature_verifies_with_its_hash_and_salt() {
    use rsa::pkcs8::EncodePublicKey;
    let mut rng = Counter(7);
    let private = rsa::RsaPrivateKey::new(&mut rng, 1024).unwrap();
    let spki = private.to_public_key().to_public_key_der().unwrap();
    let key = SubjectPublicKeyInfoOwned::from_der(spki.as_bytes()).unwrap();
    let message: &[u8] = b"lo que se firma";
    let digest = Sha256::digest(message);
    let signature = private.sign_with_rng(&mut rng, rsa::Pss::new_with_salt::<Sha256>(32), &digest).unwrap();
    // SHA-256 for the message and the mask, a salt of 32 bytes.
    let sha256 = [0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01, 0x05, 0x00];
    let sha1 = [0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00];
    let mgf1 = [0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x08];
    let parameters = |mask: &[u8]| {
        let mut mask_part = vec![0x30, (mgf1.len() + mask.len()) as u8];
        mask_part.extend_from_slice(&mgf1);
        mask_part.extend_from_slice(mask);
        let mut body = vec![0xa0, sha256.len() as u8];
        body.extend_from_slice(&sha256);
        body.extend_from_slice(&[0xa1, mask_part.len() as u8]);
        body.extend_from_slice(&mask_part);
        body.extend_from_slice(&[0xa2, 0x03, 0x02, 0x01, 0x20]);
        let mut out = vec![0x30, body.len() as u8];
        out.extend_from_slice(&body);
        AlgorithmIdentifierOwned { oid: OID_PSS, parameters: Some(der::Any::from_der(&out).unwrap()) }
    };
    let pss = parameters(&sha256);
    assert_eq!(verify(&pss, None, &key, &[message], &signature), Verified::Yes);
    assert_eq!(verify(&pss, None, &key, &[b"otra cosa"], &signature), Verified::No);
    // A mask with another hash is not read.
    assert_eq!(verify(&parameters(&sha1), None, &key, &[message], &signature), Verified::Unknown);
}

/// The PDFs of a base (a copy, `MICELYA_DOCBASE_COPY`) or of folders
/// (`MICELYA_DOCBASE_CHECK`, separated by `;`): how many are signed, how
/// many verify and whose seal they are. Only numbers, and the
/// fingerprints of the authorities the chains reach (public certificates).
///
/// ```text
/// MICELYA_DOCBASE_COPY="C:\tmp\private.sqlite3" \
///     cargo test --lib docbase::similar::seal -- --ignored --nocapture --test-threads=1
/// ```
#[test]
#[ignore = "reads the PDFs named by MICELYA_DOCBASE_COPY or MICELYA_DOCBASE_CHECK"]
fn signatures_of_real_pdfs() {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    let mut paths: Vec<PathBuf> = Vec::new();
    if let Ok(copy) = std::env::var("MICELYA_DOCBASE_COPY") {
        let conn = rusqlite::Connection::open_with_flags(&copy, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut statement = conn
            .prepare(
                "SELECT o.location, min(l.rel_path) FROM location l JOIN origin o ON o.id = l.origin_id
                 WHERE l.ext = 'pdf' AND l.file_hash IS NOT NULL GROUP BY l.file_hash",
            )
            .unwrap();
        let rows: Vec<(String, String)> =
            statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
        use crate::source_volume::SourceProvider;
        paths.extend(
            rows.into_iter()
                .filter_map(|(root, rel)| crate::source_volume::LocalFolderProvider::new(root).local_path(&rel)),
        );
    } else if let Ok(folders) = std::env::var("MICELYA_DOCBASE_CHECK") {
        fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
                    out.push(path);
                }
            }
        }
        for folder in folders.split(';').filter(|f| !f.trim().is_empty()) {
            walk(std::path::Path::new(folder.trim()), &mut paths);
        }
    } else {
        println!("neither MICELYA_DOCBASE_COPY nor MICELYA_DOCBASE_CHECK is set: nothing to measure");
        return;
    }
    let catalog = crate::docbase::similar::types::catalog();
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    let mut tops: BTreeMap<String, (String, usize)> = BTreeMap::new();
    let mut count = |key: String| *tally.entry(key).or_default() += 1;
    let mut read = 0;
    for path in &paths {
        if !path.exists() {
            count("pdf not reachable".into());
            continue;
        }
        read += 1;
        let Ok(signatures) = crate::docbase::similar::marks::pdf_signatures(path, None) else {
            count("pdf not readable".into());
            continue;
        };
        if signatures.is_empty() {
            continue;
        }
        count("pdf signed".into());
        count(format!("pdf signatures: {}", signatures.len()));
        match verdict(catalog, &signatures) {
            Some(v) => count(format!("pdf verdict: {} {}{}", v.state, v.service.unwrap_or_default(), if v.later_validation { " +validation" } else { "" })),
            None => count("pdf verdict: none".into()),
        }
        // A file whose envelopes are not where their signatures say (P-337):
        // where its ranges end against its length, and what made it.
        if signatures.iter().any(|s| s.check.as_ref().is_some_and(|c| c.problem.as_deref() == Some("byte-range"))) {
            let bytes = std::fs::read(path).unwrap_or_default();
            let ends: Vec<usize> = byte_ranges(&bytes).iter().map(|r| r[2] + r[3]).collect();
            let valid = signatures.iter().filter(|s| s.check.as_ref().is_some_and(|c| c.is_valid())).count();
            count(format!("byte-range file: {} ranges readable for {} signatures, {valid} valid", ends.len(), signatures.len()));
            count(format!(
                "byte-range file: a range ends at its end {}, beyond it {}",
                ends.iter().any(|&e| e == bytes.len()),
                ends.iter().any(|&e| e > bytes.len())
            ));
            count(format!("byte-range file: made by {}", producer_of(path)));
        }
        for signature in &signatures {
            let Some(found) = &signature.check else { continue };
            count(format!("signature: {} {} covers={}", found.state, found.problem.clone().unwrap_or_default(), found.covers));
            // The authority at the top of a chain of more than one (a
            // public certificate; a chain of one is the signer's own).
            if let (Some(top), true) = (found.chain.last(), found.chain.len() > 1) {
                let entry = tops.entry(top.clone()).or_insert_with(|| (String::new(), 0));
                entry.1 += 1;
                if entry.0.is_empty() {
                    entry.0 = format!("chain of {}: {}", found.chain.len(), authority_names(path, top).join(" / "));
                }
            }
        }
    }
    println!("pdfs: {} ({} read)", paths.len(), read);
    for (key, n) in &tally {
        println!("   {key}: {n}");
    }
    println!("authorities at the top of the verified chains (sha-256 of the certificate):");
    for (fingerprint, (what, n)) in &tops {
        println!("   {fingerprint} {what}: {n}");
    }
}

/// What made a PDF, as its producer and creator say.
fn producer_of(path: &std::path::Path) -> String {
    use pdfium_render::prelude::PdfDocumentMetadataTagType as Tag;
    let _guard = crate::commands::fs::lock_pdfium();
    let Ok(pdfium) = crate::commands::fs::pdfium() else { return String::new() };
    let Ok(doc) = pdfium.load_pdf_from_file(path, None) else { return String::new() };
    let tag = |t: Tag| doc.metadata().get(t).map(|m| m.value().to_string()).unwrap_or_default();
    format!("{} | {}", tag(Tag::Producer), tag(Tag::Creator)).to_lowercase()
}

/// The names of the certificate with this fingerprint, in the envelopes
/// of a PDF.
fn authority_names(path: &std::path::Path, wanted: &str) -> Vec<String> {
    let _guard = crate::commands::fs::lock_pdfium();
    let Ok(pdfium) = crate::commands::fs::pdfium() else { return Vec::new() };
    let Ok(doc) = pdfium.load_pdf_from_file(path, None) else { return Vec::new() };
    for signature in doc.signatures().iter() {
        let contents = signature.bytes();
        let Some((outer, _)) = tlv(&contents) else { continue };
        let Some(envelope) = Envelope::read(outer.whole) else { continue };
        for raw in envelope.certificates {
            if fingerprint(raw) == wanted {
                if let Ok(cert) = Certificate::from_der(raw) {
                    return subject_names(&cert);
                }
            }
        }
    }
    Vec::new()
}
