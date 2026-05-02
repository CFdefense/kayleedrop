//! Encryption: [`IMG_PATH`], [`TXT_PATH`], [`encrypt_content_and_write`], [`encrypt`], [`decrypt_content_and_save`], [`decrypt`], [`derive_key`].

use crate::content::Content;
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use iced::widget::image::Handle;
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use std::{env::var, error::Error, fs, path::Path};

/// Relative to the process current directory (matches `data/*/…` in this repo).
pub const IMG_SRC_PATH: &str = "data/source/img.enc";
pub const TXT_SRC_PATH: &str = "data/source/txt.enc";
pub const IMG_DEST_PATH: &str = "data/destination/img.png";
pub const TXT_DEST_PATH: &str = "data/destination/txt.out";

fn ensure_parent_dir(path: &str) -> Result<(), Box<dyn Error>> {
    if let Some(dir) = Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            fs::create_dir_all(dir)?;
        }
    }
    Ok(())
}

/// encrypt_content_and_write()
///
/// To encrypt the image and text and write them
/// Will read the image from the path, encrypt it, and write it to /data/souce/img.enc
/// Will read the text, encrypt it, and write it to /data/souce/text.enc
///
/// img_path - The path to the image to encrypt
/// text - The text to encrypt
///
/// Returns a Result with the error if any
///
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

/// encrypt()
///
/// To encrypt the data using AES-256-GCM
/// Will use the key to encrypt the data
/// Will return the nonce and ciphertext together
///
/// data - The data to encrypt
/// password - Used with a random salt to derive the key
/// Returns salt + nonce + ciphertext
///
fn encrypt(content: &[u8], password: &str) -> Vec<u8> {
    // generate salt (store this with output)
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);

    // derive key
    let key = derive_key(password, &salt);

    // create cipher
    let cipher = Aes256Gcm::new_from_slice(&key).unwrap();

    // random nonce
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // encrypt
    let ciphertext = cipher.encrypt(nonce, content).unwrap();

    // package: salt + nonce + ciphertext
    [salt.to_vec(), nonce_bytes.to_vec(), ciphertext].concat()
}

/// decrypt_content()
///
/// To decrypt the fetched blobs and save them at destination.
///
/// img_blob - The image blobs to decrypt and save
/// txt_blob - The text blobs to decrypt and save
///
/// Return the Result or Error if any
///
pub fn decrypt_content_and_save(
    img_blob: &[u8],
    txt_blob: &[u8],
) -> Result<Content, Box<dyn Error>> {
    let password = var("PASSWORD").map_err(|_| "PASSWORD environment variable is not set")?;

    // decrypt
    let img_bytes = decrypt(&img_blob, &password)?;
    let txt_bytes = decrypt(&txt_blob, &password)?;

    // convert text
    let text = String::from_utf8(txt_bytes)?;

    ensure_parent_dir(IMG_DEST_PATH)?;
    ensure_parent_dir(TXT_DEST_PATH)?;

    // save the decrypted img
    fs::write(IMG_DEST_PATH, &img_bytes)?;

    // save the decrypted text
    fs::write(TXT_DEST_PATH, &text)?;

    Ok(Content {
        image_handle: Handle::from_bytes(img_bytes),
        text,
    })
}

/// decrypt()
///
/// Decrypt some encrypted content using the password
///
/// Returns the content decrypted
///
fn decrypt(blob: &[u8], password: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    // split parts
    if blob.len() < 28 {
        return Err("Invalid encrypted data".into());
    }

    let salt = &blob[0..16];
    let nonce_bytes = &blob[16..28];
    let ciphertext = &blob[28..];

    // derive same key
    let key = derive_key(password, salt);

    // recreate cipher
    let cipher = Aes256Gcm::new_from_slice(&key)?;
    let nonce = Nonce::from_slice(nonce_bytes);

    // decrypt
    let plaintext = cipher.decrypt(nonce, ciphertext).unwrap();

    Ok(plaintext)
}

/// derive_key()
///
/// To derive the key from the password and salt using PBKDF2-HMAC-SHA256
/// Will use the password and salt to derive the key
/// Will return the key
///
/// password - The password to use for key derivation
/// salt - The salt to use for key derivation
/// Returns the key
///
fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];

    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt, 100_000, &mut key);

    key
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_encrypt_and_decrypt() {}
    fn test_encrypt_writes() {}
    fn test_decrypt_writes() {}
    fn test_derive_key() {}
}
