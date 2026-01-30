# clashctl

Simple TUI controller for Clash via the External Controller API.

## Features
- Manage proxy groups and nodes from the terminal
- Batch speed test with async UI (no freeze; hidden in Work preset)
- Simple/Expert modes with quick navigation
- Subscription update (proxy-providers, Mihomo Party)
- View connections and logs

## Requirements
- Rust (for building)
- A running Clash-compatible core with External Controller enabled
- Prebuilt releases are provided for macOS (Apple Silicon / Intel)

## Quick start

```bash
cargo build --release
./run.sh

# or run directly
./target/release/clashctl --api-url http://127.0.0.1:9090 --secret your_secret
```

## Install (macOS)
1. Download the matching release asset from GitHub Releases.
   - `arm64` for Apple Silicon
   - `x86_64` for Intel
2. Unpack and move the binary into your PATH.

```bash
tar -xzf clashctl-<version>-macos-<arch>.tar.gz
chmod +x clashctl
sudo mv clashctl /usr/local/bin/
```

## Basic keys
- `g` Routes, `m` mode (Rule/Global/Direct)
- `t` speed test (Routes)
- `Enter` switch node
- `q`/`Esc` quit (with confirmation)

## Config
- Default API: `http://127.0.0.1:9090`
- CLI flags: `--api-url`, `--secret`, `--help`, `--version`
- Update page reads subscriptions from:
  - Clash config `proxy-providers`
  - Mihomo Party `profile.yaml` + `profiles/<id>.yaml`
- Override paths via `CLASH_CONFIG_PATH` and `CLASH_PARTY_DIR`
- Priority: CLI > defaults

## Docs
- `USAGE.md`

## License
MIT
