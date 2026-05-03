# KayleeDrop

Small desktop app that **checks in over the network**, pulls **AES-GCM ciphertext** blobs you host somewhere public (defaults: raw GitHub URLs in the binary), decrypts them with a shared **`PASSWORD`**, and pops an **iced** window with your image plus caption **only when the remote bundle is new** versus what’s already decrypted on disk—so it behaves a bit like an **encrypted poke / ping**, not a constant nag.

Originally aimed at harmless chaos on a **MacBook Neo** (Apple Silicon): install the LaunchAgent curl one-liner, she leaves the laptop on with the agent loaded, you rotate **`img.enc` / `txt.enc`** upstream; next scheduled run—or manual launch—she gets the update if passwords still match.

**Only mess with machines and people where everyone’s in on the joke.** This is toy software, not covert tooling.

---

## What it actually does

1. **Scheduled or manual runs** (`install-service.sh` sets up systemd on Linux or LaunchAgent/LaunchDaemon on macOS).
2. **HTTP GET** of two ciphertext files (image + text).
3. **Decrypt** with `PASSWORD` (from env or `.env`; see `.env.example`).
4. Compare decrypted output to files under **`data/destination/`**. If unchanged, exit quietly—no flash, no tray spam.
5. If something new decrypts cleanly, **open one GUI window** with the raster and caption until dismissed.

Separate **encrypt** mode bundles a local PNG and string into **`data/source/`** for you to publish.

```bash
# After PASSWORD is set (env or `.env` in cwd):
cargo run -- /path/to/image.png "Caption goes here"
```

Then push/sync **`data/source/img.enc`** and **`txt.enc`** to whatever URLs you wired in **`src/content/mod.rs`** (or fork and change **`REMOTE_IMG_URL`** / **`REMOTE_TEXT_URL`**).

GUI mode:

```bash
PASSWORD='…' cargo run
```

---

## macOS (Neo / M-series, no sudo)

```bash
curl -fsSL https://raw.githubusercontent.com/CFdefense/kayleedrop/main/scripts/install-service.sh | bash -s --
```

Run that **one** line exactly (pastes that glue two `curl … | bash` copies together confuse `bash` and yield `bash: --: invalid option`). The installer creates **`PASSWORD`** under **`<install root>/.env`** (typically `~/Library/Application Support/KayleeDrop/.env`). Older installs used a file named `env` next to `.env`; the LaunchAgent loads **`.env` first**, then falls back to `env`. Default daily fire is **09:00** local; override with **`LAUNCHD_HOUR`** and **`LAUNCHD_MINUTE`** if you curl with env on the pipe.

### LaunchAgent sees no `PASSWORD` (`/tmp/kayleedrop.err`)

1. **Uncomment assignment** — the line must be real shell, not `# PASSWORD=…`; use `PASSWORD=your-secret` or `export PASSWORD=your-secret`.

2. **Ownership and readability** — the agent runs as you; **`sudo`** editing can leave root-owned unreadable secrets:

   ```bash
   ls -la ~/Library/Application\ Support/KayleeDrop/.env
   ```

   If needed: `chmod 600` and ownership back to your user.

3. **Path matches plist** — confirm the plist’s `source …` targets the same file you edit:

   ```bash
   plutil -p ~/Library/LaunchAgents/io.github.cfdefense.kayleedrop.plist
   ```

4. **Reproduce without launchd** (substitute **`ROOT`** if your install differs):

   ```bash
   /bin/bash -lc '
   set -euo pipefail
   ROOT="$HOME/Library/Application Support/KayleeDrop"
   _loaded=0
   if [[ -r "$ROOT/.env" ]]; then set -a; source "$ROOT/.env"; set +a; _loaded=1; fi
   if [[ "$_loaded" -eq 0 ]] && [[ -r "$ROOT/env" ]]; then set -a; source "$ROOT/env"; set +a; fi
   if [[ -n "${PASSWORD+x}" ]]; then echo "PASSWORD is set (length ${#PASSWORD})"; else echo "PASSWORD not exported"; fi
   '
   ```

After changing `scripts/install-service.sh`, run the installer again or edit the plist so **`ProgramArguments`** matches the script (launchd ignores repo changes until plist is rewritten).

Intel Mac builds aren’t shipped in CI unless you add a job or set **`BINARY_URL`** for a custom tarball.

---

## Building

Needs a normal Rust toolchain (edition 2024). On macOS/Linux:

```bash
cargo build --release
```

Release artifacts (**linux amd64/arm64**, **darwin arm64**) are built in **`release-binaries`** when you publish a GitHub Release.

---

## Stack (short)

Rust, **iced**, **reqwest**, **AES-GCM**, **PBKDF2**. Toy project; don’t reuse the crypto story for production secrets without a real threat model review.
