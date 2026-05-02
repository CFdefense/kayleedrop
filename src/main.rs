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

use crate::content::{
    Content, REMOTE_IMG_URL, REMOTE_TEXT_URL, remote_plaintext_matches_destination,
};
use iced::{Size, window};

/// Events dispatched from widgets or async loaders into iced’s `update` path.
///
/// Carries caption edits plus optional replacement [`Content`] from a background fetch.
#[derive(Debug, Clone)]
pub enum Message {
    /// Updates the caption string shown below the raster.
    ///
    /// Does not persist to disk unless the encrypt CLI is run again.
    UpdateText(String),

    /// Delivers decrypted remote content (success) or discards failures in [`crate::app::update`].
    ContentLoaded(Result<Content, String>),
}

/// Builds the initial iced window size around the decrypted bitmap plus space for caption text.
///
/// Width and height come from [`Content::image_size`]. Adds a fixed vertical band for one line of
/// UI text so the inner client area fits image + caption without clipping.
///
/// See also [`iced::window::Settings`].
fn window_settings_for_content(c: &Content) -> window::Settings {
    const SPACING: f32 = 8.0;
    const CAPTION_ROWS: f32 = 26.0;
    let (w, h) = c.image_size;
    window::Settings {
        size: Size::new(w as f32, h as f32 + SPACING + CAPTION_ROWS),
        ..window::Settings::default()
    }
}

/// Prints encrypt / GUI usage to standard error.
///
/// The caller terminates the process with code `64` for invalid arity; this helper only emits the
/// message body.
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

/// Program entrypoint: routes to GUI mode (`argv.len() == 1`), encrypt CLI (`3` arguments), or help + exit `64`.
fn main() -> Result<(), Box<dyn Error>> {
    // Load `.env` into the process env
    dotenvy::dotenv().ok();

    let args: Vec<String> = args().collect();
    let bin = args.first().map(String::as_str).unwrap_or("kayleedrop");

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

/// Runs interactive mode after remote / local consistency checks succeed.
///
/// Exits quietly when [`content::remote_plaintext_matches_destination`] reports no new content, when
/// `PASSWORD` is missing, or when the remote fetch/decrypt fails. Otherwise builds an iced
/// application seeded with decrypted [`Content`] and blocks until the window closes.
///
/// # Errors
///
/// Returns boxed errors mainly from Tokio blocking on the runtime or iced startup failures.
fn run_gui() -> Result<(), Box<dyn Error>> {
    let rt = Runtime::new()?;

    // Remote decrypts equal local `data/destination/` outputs ⇒ nothing new to show
    if rt.block_on(remote_plaintext_matches_destination())? {
        eprintln!("Remote content matches decrypted files under data/destination/; exiting.");
        return Ok(());
    }

    // Need PASSWORD to fetch/decrypt (compare step already required it if destinations existed)
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

    let iced_window_layout = window_settings_for_content(&content);
    let initial_content = Mutex::new(Some(content));
    let iced_window = iced::application(
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
    .window(iced_window_layout)
    .centered();

    iced_window
        .title(|state: &MyApp| state.title.clone())
        .run()
        .map_err(|e| -> Box<dyn Error> { e.into() })?;

    Ok(())
}
