//! Identity of a source file by its bytes.
//!
//! Change detection has its own cheap hash to notice that a file changed.
//! Saying that two files *are the same document* is a stronger claim, so
//! it uses SHA-256.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

/// Files above this size are not hashed; they are simply never matched
/// by their bytes.
pub const MAX_HASHED_BYTES: u64 = 512 * 1024 * 1024;

/// SHA-256 of the file, in lowercase hex. `Ok(None)` when the file is
/// too large to be worth hashing.
pub fn file_sha256(path: &Path) -> Result<Option<String>, String> {
    let mut file =
        File::open(path).map_err(|e| format!("Failed to open '{}': {e}", path.display()))?;
    let size = file
        .metadata()
        .map_err(|e| format!("Failed to read metadata of '{}': {e}", path.display()))?
        .len();
    if size > MAX_HASHED_BYTES {
        return Ok(None);
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Failed to read '{}': {e}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(Some(hex(&hasher.finalize())))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_bytes_give_the_same_hash_and_one_changed_byte_another() {
        let dir = std::env::temp_dir().join(format!("micelya-file-hash-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.jpg"), b"same bytes").unwrap();
        std::fs::write(dir.join("renamed copy.jpg"), b"same bytes").unwrap();
        std::fs::write(dir.join("b.jpg"), b"same bytez").unwrap();

        let a = file_sha256(&dir.join("a.jpg")).unwrap().unwrap();
        assert_eq!(a.len(), 64);
        assert_eq!(Some(a.clone()), file_sha256(&dir.join("renamed copy.jpg")).unwrap());
        assert_ne!(Some(a), file_sha256(&dir.join("b.jpg")).unwrap());
        assert!(file_sha256(&dir.join("missing.jpg")).is_err());

        let _ = std::fs::remove_dir_all(dir);
    }
}
