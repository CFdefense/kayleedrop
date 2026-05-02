//! AES-256-GCM encryption helpers, path constants, and PNG IHDR parsing for window sizing.
//!
//! Public entry points: [`encrypt_content_and_write`], [`decrypt_content_and_save`], [`decrypt`],
//! [`content_from_plaintext`].

use crate::content::Content;
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use iced::widget::image::Handle;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use std::{env::var, error::Error, fs, path::Path};

/// Encrypted image blob path (relative to process working directory).
pub const IMG_SRC_PATH: &str = "data/source/img.enc";
/// Encrypted text blob path (relative to process working directory).
pub const TXT_SRC_PATH: &str = "data/source/txt.enc";
/// Decrypted image output path.
pub const IMG_DEST_PATH: &str = "data/destination/img.png";
/// Decrypted caption output path.
pub const TXT_DEST_PATH: &str = "data/destination/txt.out";

/// Extracts IHDR dimensions from an in-memory PNG without decoding scanlines.
///
/// Assumes a standards-compliant PNG where the first chunk after the magic signature is IHDR. Returns
/// [`None`] when the signature mismatches, the payload is truncated, dimensions parse as zero, or the
/// structure is malformed.
///
/// Intended for sizing iced windows—not a general-purpose PNG validator.
fn png_pixel_size(bytes: &[u8]) -> Option<(u32, u32)> {
    const SIG: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if bytes.len() < 24 {
        return None;
    }
    let head = bytes.get(0..8)?;
    if head != SIG.as_slice() {
        return None;
    }
    let w = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let h = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

/// Recursively ensures the parent folder for `path` exists before writing ciphertext / plaintext blobs.
///
/// No-ops cleanly when no parent (`"relative.txt"` style) applies. Intended for bundled `data/...`
/// hierarchies tracked in-repo.
fn ensure_parent_dir(path: &str) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            fs::create_dir_all(dir)?;
        }
    }
    Ok(())
}

/// CLI helper invoked as `exe <PNG> "<caption>"` when three arguments are supplied.
///
/// Reads the plaintext image bytes from `img_path`, encrypts BOTH blobs with PBKDF2 + AES-GCM keyed by
/// the `"PASSWORD"` environment variable, writes [`IMG_SRC_PATH`] / [`TXT_SRC_PATH`], creating parent dirs
/// on demand.
///
/// # Errors
///
/// Bubbled when `"PASSWORD"` is unset, filesystem reads fail on the PNG, or ciphertext cannot be flushed
/// atomically beneath `data/source`.
pub fn encrypt_content_and_write(img_path: &str, text: &str) -> Result<(), Box<dyn Error>> {
    let password = var("PASSWORD").map_err(|_| "PASSWORD environment variable is not set")?;

    let img = fs::read(img_path)
        .map_err(|e| -> Box<dyn Error> { format!("cannot read image `{img_path}`: {e}").into() })?;

    let img_encrypted = encrypt(&img, &password);
    let txt_encrypted = encrypt(text.as_bytes(), &password);

    ensure_parent_dir(IMG_SRC_PATH)?;
    ensure_parent_dir(TXT_SRC_PATH)?;
    fs::write(IMG_SRC_PATH, img_encrypted)?;
    fs::write(TXT_SRC_PATH, txt_encrypted)?;

    Ok(())
}

/// Builds the on-wire ciphertext framing shared by BOTH image and caption blobs.
///
/// Layout concatenates, in order:
///
/// - 16-byte PBKDF2 salt
/// - 12-byte AES-GCM nonce (`IV`)
/// - ciphertext octets authenticated with the GMAC tag appended by AES-GCM
///
/// Randomness derives from [`rand::thread_rng`]; ciphertext differs every invocation even for identical plaintext.
fn encrypt(content: &[u8], password: &str) -> Vec<u8> {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);

    let key = derive_key(password, &salt);

    let cipher = Aes256Gcm::new_from_slice(&key).unwrap();

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, content).unwrap();

    [salt.to_vec(), nonce_bytes.to_vec(), ciphertext].concat()
}

/// End-to-end path used during [`crate::content::Content::fetch_blocking`] and GUI startup workflows.
///
/// Accepts ciphertext slices (typically fetched over HTTP), decrypts BOTH using `"PASSWORD"` from the
/// environment snapshot, persists PNG + UTF-8 text under [`IMG_DEST_PATH`] / [`TXT_DEST_PATH`],
/// infers PNG dimensions from the IHDR chunk (fallback `(256, 256)` outside PNG), and packages an
/// iced-friendly [`crate::content::Content`].
///
/// # Errors
///
/// Same classes as [`decrypt`] (bad password/mac), `"PASSWORD"` omissions, caption UTF-8 issues, as
/// well as inability to mkdir/write under `data/destination`.
pub fn decrypt_content_and_save(
    img_blob: &[u8],
    txt_blob: &[u8],
) -> Result<Content, Box<dyn Error>> {
    let password = var("PASSWORD").map_err(|_| "PASSWORD environment variable is not set")?;

    let img_bytes = decrypt(img_blob, &password)?;
    let txt_bytes = decrypt(txt_blob, &password)?;

    let text = String::from_utf8(txt_bytes)?;

    ensure_parent_dir(IMG_DEST_PATH)?;
    ensure_parent_dir(TXT_DEST_PATH)?;

    fs::write(IMG_DEST_PATH, &img_bytes)?;
    fs::write(TXT_DEST_PATH, &text)?;

    Ok(content_from_plaintext(&img_bytes, text))
}

/// Lightweight constructor for tests and for post-decrypt assembly without extra I/O.
///
/// Accepts raw image bytes (typically a PNG) plus the already-decoded UTF-8 caption. When the bytes
/// lack a valid IHDR header, the returned [`crate::content::Content`] uses `(256, 256)` for its
/// `image_size` tuple so iced still receives reasonable window hints.
pub fn content_from_plaintext(img_plain: &[u8], text: String) -> Content {
    let image_size = png_pixel_size(img_plain).unwrap_or((256, 256));
    Content::new(
        Handle::from_bytes(img_plain.to_vec()),
        image_size,
        text,
    )
}

/// Reconstructs plaintext from the serialized layout emitted by the internal encrypt routine.
///
/// Splits the fixed header (salt + nonce) from the AEAD payload, re-derives the AES-256 key (100k
/// PBKDF2-HMAC-SHA256 iterations), verifies the authentication tag, then returns the inner bytes on
/// success.
///
/// # Errors
///
/// Returns boxed errors for truncated buffers, password/tag mismatches (`decrypt failed ...`), or malformed
/// ciphertext that cannot be deserialized by `aes-gcm`.
pub fn decrypt(blob: &[u8], password: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    const MIN: usize = 28;
    if blob.len() < MIN {
        return Err(format!(
            "invalid or truncated ciphertext: got {} bytes, need at least {} (wrong URL/file, plaintext 404/HTML, or not our binary format)",
            blob.len(),
            MIN
        )
        .into());
    }

    let salt = &blob[0..16];
    let nonce_bytes = &blob[16..28];
    let ciphertext = &blob[28..];

    let key = derive_key(password, salt);

    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| -> Box<dyn Error> {
            "decrypt failed (wrong PASSWORD or corrupt ciphertext)".into()
        })?;

    Ok(plaintext)
}

/// Derives the 32-byte AES key used by both [`encrypt`] and [`decrypt`].
///
/// Wraps PBKDF2-HMAC-SHA256 (`100_000` iterations) keyed by `password` / `salt`.
///
/// Deterministic for identical inputs, which keeps unit tests reproducible.
fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut key);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_PNG: &[u8] = include_bytes!("../../tests/fixtures/3x7.png");

    #[test]
    fn png_pixel_size_reads_ihdr() {
        assert_eq!(png_pixel_size(FIXTURE_PNG), Some((3, 7)));
    }

    #[test]
    fn png_pixel_size_rejects_non_png() {
        assert!(png_pixel_size(b"not a png").is_none());
        assert!(png_pixel_size(&FIXTURE_PNG[..20]).is_none());
    }

    #[test]
    fn derive_key_is_deterministic() {
        let salt = [1u8; 16];
        let a = derive_key("pw", &salt);
        let b = derive_key("pw", &salt);
        assert_eq!(a, b);
        assert_ne!(derive_key("other", &salt), a);
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let plaintext = b"payload bytes \xff";
        let enc = encrypt(plaintext, "secret");
        let out = decrypt(&enc, "secret").unwrap();
        assert_eq!(out.as_slice(), plaintext);
    }

    #[test]
    fn decrypt_wrong_password_fails() {
        let enc = encrypt(b"x", "good");
        assert!(decrypt(&enc, "bad").is_err());
    }

    #[test]
    fn decrypt_truncated_returns_error() {
        let short = vec![0u8; 10];
        assert!(decrypt(&short, "pw").is_err());
    }

    #[test]
    fn content_from_plaintext_uses_png_dimensions() {
        let c = content_from_plaintext(FIXTURE_PNG, "hi".into());
        assert_eq!(c.image_size, (3, 7));
        assert_eq!(c.text, "hi");
    }

    #[test]
    fn content_from_plaintext_non_png_fallback_size() {
        let c = content_from_plaintext(b"jpeg-ish", "".into());
        assert_eq!(c.image_size, (256, 256));
    }

    #[test]
    fn roundtrip_decrypt_rebuilds_content() {
        let pwd = "integration-pw";
        let img = FIXTURE_PNG;
        let enc_img = encrypt(img, pwd);
        let enc_txt = encrypt(b"caption", pwd);
        let plain_img = decrypt(&enc_img, pwd).unwrap();
        assert_eq!(plain_img.as_slice(), img);
        let plain_txt = decrypt(&enc_txt, pwd).unwrap();
        let rebuilt = content_from_plaintext(&plain_img, String::from_utf8(plain_txt).unwrap());
        assert_eq!(rebuilt.image_size, (3, 7));
        assert_eq!(rebuilt.text, "caption");
    }

}
