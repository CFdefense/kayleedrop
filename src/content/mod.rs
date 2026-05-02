//! Remote ciphertext URLs and the [`Content`] bundle used by the GUI loader.
//!
//! See [`Content::fetch_blocking`] and gate helper [`remote_plaintext_matches_destination`].

use crate::encryption::{IMG_DEST_PATH, TXT_DEST_PATH, decrypt, decrypt_content_and_save};
use iced::widget::image;
use reqwest;
use std::env::var;
use std::error::Error;
use std::fs;

/// GitHub URLs for ciphertext that mirror [`encryption::IMG_SRC_PATH`] /
/// [`encryption::TXT_SRC_PATH`] (**`txt.enc`**, never `text.enc`).
pub const REMOTE_IMG_URL: &str =
    "https://raw.githubusercontent.com/CFdefense/kayleedrop/main/data/source/img.enc";
pub const REMOTE_TEXT_URL: &str =
    "https://raw.githubusercontent.com/CFdefense/kayleedrop/main/data/source/txt.enc";

/// Bundle of decrypted image handle, intrinsic PNG size (when applicable), and caption text.
#[derive(Clone, Debug)]
pub struct Content {
    pub image_handle: image::Handle,
    /// PNG width × height from IHDR, or fallback when the blob is not a PNG.
    pub image_size: (u32, u32),
    pub text: String,
}

impl Default for Content {
    /// Returns a trivial 1×1 transparent raster handle and empty caption (useful only in tests or placeholders).
    fn default() -> Self {
        Self::new(
            image::Handle::from_rgba(1, 1, vec![0u8; 4]),
            (1, 1),
            String::new(),
        )
    }
}

impl Content {
    /// Wraps decrypted assets for rendering / window sizing hints.
    ///
    /// # Arguments
    ///
    /// - `image_size` — PNG IHDR geometry when known; iced uses this for native window sizing.
    /// - `image_handle` — typically produced by [`image::Handle::from_bytes`] after decryption.
    pub fn new(image_handle: image::Handle, image_size: (u32, u32), text: String) -> Self {
        Self {
            image_handle,
            image_size,
            text,
        }
    }

    /// Blocking-friendly async entry that mirrors the bundled GitHub payloads.
    ///
    /// Hits `img_hook`/`text_hook` with `GET`, rejects empty bodies, decrypts blobs using the ambient
    /// [`std::env::var`] `"PASSWORD"` (usually loaded via `dotenvy`), persists plaintext beneath
    /// [`crate::encryption::IMG_DEST_PATH`] / [`crate::encryption::TXT_DEST_PATH`], and finally wraps
    /// the results in [`Content`].
    ///
    /// # Errors
    ///
    /// Covers HTTP/stack failures when downloading, decryption/AEAD mismatches when the password does
    /// not match, UTF-8 issues in the decrypted caption, or filesystem persistence errors surfaced by
    /// [`crate::encryption::decrypt_content_and_save`].
    pub async fn fetch_blocking(
        img_hook: &str,
        text_hook: &str,
    ) -> Result<Content, Box<dyn Error>> {
        // get the encrpyted image from the remote store
        let img_response = reqwest::get(img_hook).await?;
        let img_body = img_response.bytes().await?;

        // get the encrypted text
        let txt_response = reqwest::get(text_hook).await?;
        let txt_body = txt_response.bytes().await?;

        if img_body.is_empty() || txt_body.is_empty() {
            return Err("remote returned empty encrypted payload".into());
        }

        // decrypt and save the contents
        let result = decrypt_content_and_save(img_body.as_ref(), txt_body.as_ref())?;

        Ok(result)
    }
}

/// Compares freshly downloaded ciphertext with what was last decrypted to disk.
///
/// When `"PASSWORD"` is present, decrypts BOTH remote payloads and BOTH destination files produced by an
/// earlier successful run (see [`Content::fetch_blocking`]). Returns `Ok(true)` only when BOTH image/caption
/// byte strings match pairwise.
///
/// Any missing env var, unreadable destinations, networking issues, decryption failures, or mismatch
/// results in `Ok(false)` unless a hard filesystem / HTTP failure surfaces as `Err(...)`.
///
/// # Errors
///
/// Network errors from [`reqwest`] or non-missing read failures (permissions, etc.).
pub async fn remote_plaintext_matches_destination() -> Result<bool, Box<dyn Error>> {
    let password = match var("PASSWORD") {
        Ok(p) => p,
        Err(_) => return Ok(false),
    };

    let dest_img = match fs::read(IMG_DEST_PATH) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };
    let dest_txt = match fs::read(TXT_DEST_PATH) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };

    let remote_img = reqwest::get(REMOTE_IMG_URL).await?.bytes().await?;
    let remote_txt = reqwest::get(REMOTE_TEXT_URL).await?.bytes().await?;

    if remote_img.is_empty() || remote_txt.is_empty() {
        return Ok(false);
    }

    let img_plain = match decrypt(remote_img.as_ref(), &password) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };
    let txt_plain = match decrypt(remote_txt.as_ref(), &password) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };

    Ok(img_plain == dest_img && txt_plain == dest_txt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_content_minimal_dimensions() {
        let c = Content::default();
        assert_eq!(c.image_size, (1, 1));
        assert!(c.text.is_empty());
    }

    #[test]
    fn new_preserves_fields() {
        let h = image::Handle::from_bytes(vec![9, 9, 9]);
        let c = Content::new(h.clone(), (10, 20), "a".into());
        assert_eq!(c.image_size, (10, 20));
        assert_eq!(c.text, "a");
    }
}
