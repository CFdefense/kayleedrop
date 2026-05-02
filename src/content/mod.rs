//! Content: [`REMOTE_IMG_URL`], [`REMOTE_TEXT_URL`], [`Content`], [`Content::fetch_blocking`], [`remote_bytes_match_local_files`].

use crate::encryption::{IMG_SRC_PATH, TXT_SRC_PATH, decrypt_content_and_save};
use iced::widget::image;
use reqwest;
use std::error::Error;
use std::fs;

/// HTTP sources for remote content (must match what [`Content::fetch`] uses).
pub const REMOTE_IMG_URL: &str =
    "https://raw.githubusercontent.com/CFdefense/maybe-malware/data/source/img.enc";
pub const REMOTE_TEXT_URL: &str =
    "https://raw.githubusercontent.com/CFdefense/maybe-malware/data/source/text.enc";

/// Content
///
/// Contains built iced image and text widgets to render and display
///
#[derive(Clone, Debug)]
pub struct Content {
    pub image_handle: image::Handle,
    pub text: String,
}

impl Default for Content {
    fn default() -> Self {
        Self::new(image::Handle::from_rgba(1, 1, vec![0u8; 4]), String::new())
    }
}

impl Content {
    /// new()
    ///
    /// Initialize an instance of Content
    /// To hold an image handle and text
    ///
    pub fn new(image_handle: image::Handle, text: String) -> Self {
        Content { image_handle, text }
    }

    /// fetch()
    ///
    /// To fetch and initialize the content, will build the iced widgets
    /// Will attempt to fetch the content and store the image
    ///
    pub async fn fetch_blocking(
        img_hook: &str,
        text_hook: &str,
    ) -> Result<Content, Box<dyn Error>> {
        // get the encrpyted image in base64 format
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

        // build the content
        Ok(result)
    }
}

/// `true` when remote ciphertext matches on-disk ciphertext at [`encryption::IMG_SRC_PATH`] /
/// [`encryption::TXT_SRC_PATH`] (what [`encryption::encrypt_content_and_write`] produces).
///
/// Missing local files ⇒ `false` (treat as "differs").
pub async fn remote_bytes_match_local_files() -> Result<bool, Box<dyn Error>> {
    let remote_img = reqwest::get(REMOTE_IMG_URL).await?.bytes().await?;
    let remote_text = reqwest::get(REMOTE_TEXT_URL).await?.bytes().await?;

    let local_img = match fs::read(IMG_SRC_PATH) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };
    let local_text = match fs::read(TXT_SRC_PATH) {
        Ok(b) => b,
        Err(_) => return Ok(false),
    };

    Ok(
        remote_img.as_ref() == local_img.as_slice()
            && remote_text.as_ref() == local_text.as_slice(),
    )
}

#[cfg(test)]
mod tests {}
