# KayleeDrop

Fetches two remote ciphertext blobs (defaults in `src/content/mod.rs`), decrypts with **`PASSWORD`**, compares to **`data/destination/`**, opens an **iced** window only when remote plaintext differs. **Encrypt** mode writes **`data/source/img.enc`** and **`txt.enc`**. Toy project; use only where everyone consents.

## Install

**macOS (user LaunchAgent, no sudo)** — use raw **`CFdefense/kayleedrop`** (not stale mirrors):

```bash
curl -fsSL https://raw.githubusercontent.com/CFdefense/kayleedrop/main/scripts/install-service.sh | bash -s --
```

Use a **single** `curl … | bash -s --` (no doubled `--`).

- **Binary / data:** `~/Library/Application Support/KayleeDrop/` (default).
- **Secrets:** **`…/KayleeDrop/.env`** (line must be `PASSWORD=…`, not `# PASSWORD=…`). Legacy **`…/env`** still works if `.env` is absent.
- **Schedule:** daily **`LAUNCHD_HOUR`/`LAUNCHD_MINUTE`** in **system local time** (defaults **22:00**). Set macOS timezone to **America/New_York** for Eastern evening. Overrides: `LAUNCHD_HOUR=9 LAUNCHD_MINUTE=0 curl … | bash -s --`. Dev interval: `LAUNCHD_START_INTERVAL=30 bash …` (reinstall without it for calendar mode).
- **Logs:** `/tmp/kayleedrop.out`, `/tmp/kayleedrop.err`. Reload after edits: `launchctl bootout "gui/$(id -u)" "$HOME/Library/LaunchAgents/io.github.cfdefense.kayleedrop.plist" && launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/io.github.cfdefense.kayleedrop.plist"`.

**Linux:** `sudo bash` the same script (systemd timer + `/opt/kayleedrop`, `/etc/kayleedrop.env`).

**Uninstall:** `bash scripts/uninstall-service.sh` or `curl -fsSL https://raw.githubusercontent.com/CFdefense/kayleedrop/main/scripts/uninstall-service.sh | bash -s --` (macOS user: no sudo; Linux / macOS daemon: **`sudo`**). **`--purge`** removes **`INSTALL_ROOT`**.

## If `PASSWORD` is missing

Check **`ls -la ~/Library/Application\ Support/KayleeDrop/.env`** (readable by you, not root-owned). **`plutil -p ~/Library/LaunchAgents/io.github.cfdefense.kayleedrop.plist`** to confirm paths. Re-run the installer after changing **`install-service.sh`**.

**Intel macOS:** set **`BINARY_URL`** to a tarball or build from source.

## From source

```bash
# .env or env with PASSWORD=… in cwd
cargo run -- /path/to/image.png "caption"
PASSWORD='…' cargo run   # GUI
cargo build --release
```

Publish **`data/source/*.enc`** to the URLs in **`src/content/mod.rs`** (or change **`REMOTE_*_URL`** and rebuild). Release workflow: **`.github/workflows/release-binaries.yml`**.

Stack: Rust, iced, reqwest, AES-GCM, PBKDF2.
