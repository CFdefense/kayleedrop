mod app;
mod content;
mod encryption;

use app::{MyApp, boot_with_content, update, view};
use encryption::encrypt_content_and_write;
use std::env::args;
use std::env::var;
use std::error::Error;
use std::process;
use std::sync::Mutex;
use tokio::runtime::Runtime;

use crate::content::{Content, REMOTE_IMG_URL, REMOTE_TEXT_URL, remote_bytes_match_local_files};

/// Message
///
/// An iced Message
///
#[derive(Debug, Clone)]
pub enum Message {
    UpdateText(String),
    ContentLoaded(Result<Content, String>),
}

fn print_encrypt_usage(bin: &str) {
    eprintln!(
        "\
Usage:
  {bin}
      Run the GUI (remote fetch/decrypt gates apply).

  {bin} <IMAGE_PATH> <TEXT>
      Read the image from disk and encrypt it with TEXT; write ciphertext to bundled paths \
(requires PASSWORD in the environment or a `.env` file).
",
        bin = bin,
    );
}

fn main() -> Result<(), Box<dyn Error>> {
    // Load `.env` into the process env
    dotenvy::dotenv().ok();

    let args: Vec<String> = args().collect();
    let bin = args.first().map(String::as_str).unwrap_or("maybe-malware");

    match args.len() {
        1 => run_gui(),
        3 => {
            encrypt_content_and_write(&args[1], &args[2])?;
            Ok(())
        }
        _ => {
            print_encrypt_usage(bin);
            process::exit(64);
        }
    }
}

fn run_gui() -> Result<(), Box<dyn Error>> {
    let rt = Runtime::new()?;

    // Remote same as bundled encrypted files — no decrypt / GUI
    if rt.block_on(remote_bytes_match_local_files())? {
        eprintln!("Remote content unchanged from local encrypted files; exiting.");
        return Ok(());
    }

    // Need PASSWORD only when we actually fetch/decrypt fresh ciphertext
    if var("PASSWORD").is_err() {
        eprintln!("PASSWORD environment variable is not set; not starting GUI.");
        return Ok(());
    }

    // Fetch + decrypt; any failure ⇒ no GUI
    let content = match rt.block_on(Content::fetch_blocking(REMOTE_IMG_URL, REMOTE_TEXT_URL)) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to load remote content: {e}");
            return Ok(());
        }
    };

    let initial_content = Mutex::new(Some(content));
    iced::application(
        move || {
            boot_with_content(
                initial_content
                    .lock()
                    .expect("mutex poisoned")
                    .take()
                    .expect("iced boot invoked more than once"),
            )
        },
        update,
        view,
    )
    .title(|state: &MyApp| state.title.clone())
    .run()
    .map_err(|e| -> Box<dyn Error> { e.into() })?;

    Ok(())
}
