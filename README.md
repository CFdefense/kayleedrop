# KayleeDrop

A small, automated “drop” system that periodically fetches encrypted content, decrypts it locally with a shared password, and displays it (image/text) on the target machine. It’s less like messaging in the traditional sense and more like a scheduled, one-way encrypted content delivery system.

Originally designed to send my girlfriend goofy pictures occasionally.

## Install

**macOS (user LaunchAgent, no sudo)** — use raw **`CFdefense/kayleedrop`** (not stale mirrors):

```bash
curl -fsSL https://raw.githubusercontent.com/CFdefense/kayleedrop/main/scripts/install-service.sh | bash -s --
```

- **Binary / data:** `~/Library/Application Support/KayleeDrop/` (default).
- **Secrets:** **`…/KayleeDrop/.env`** (line must be `PASSWORD=…`, not `# PASSWORD=…`).
- **Schedule:** daily **`LAUNCHD_HOUR`/`LAUNCHD_MINUTE`** in **system local time** (defaults **22:00**). Set macOS timezone to **America/New_York** for Eastern evening.
- **Logs:** `/tmp/kayleedrop.out`, `/tmp/kayleedrop.err`. Reload after edits: `launchctl bootout "gui/$(id -u)" "$HOME/Library/LaunchAgents/io.github.cfdefense.kayleedrop.plist" && launchctl bootstrap "gui/$(id -u)" "$HOME/Library/LaunchAgents/io.github.cfdefense.kayleedrop.plist"`.

**Linux:** `sudo bash` the same script (systemd timer + `/opt/kayleedrop`, `/etc/kayleedrop.env`).

**Uninstall:** `bash scripts/uninstall-service.sh` or `curl -fsSL https://raw.githubusercontent.com/CFdefense/kayleedrop/main/scripts/uninstall-service.sh | bash -s --` (macOS user: no sudo; Linux / macOS daemon: **`sudo`**). **`--purge`** removes **`INSTALL_ROOT`**.

**Intel macOS:** set **`BINARY_URL`** to a tarball or build from source.

## From source

```bash
# .env or env with PASSWORD=… in cwd
cargo run -- /path/to/image.png "caption"
PASSWORD='…' cargo run   # GUI
cargo build --release
```

Publish **`data/source/*.enc`** to the URLs in **`src/content/`** (or change **`REMOTE_*_URL`** and rebuild). 

Release workflow: **`.github/workflows/release-binaries.yml`**.

Stack: Rust, iced, reqwest, AES-GCM, PBKDF2.
