---
name: kobo-build
description: Cross-compile and deploy the Kobo e-reader binary to device (build, FTP deploy, HTTP pull deploy, MD5 verify)
---

# kobo-build

Build and deploy the Kobo e-reader binary to a Kobo Libra Colour device.

## Project layout

- `kothok/` — Rust workspace root
- `kothok/crates/kobo/` — device binary (`-p kobo`)
- `kothok/crates/kobo-audio/` — audio primitives
- `kothok/crates/kothok-core/` — pure logic (desktop testable)
- `kothok/crates/sim/` — desktop simulator (Slint + winit)
- `smoke/` — standalone smoke test binary

## Build

```powershell
cross build --target armv7-unknown-linux-musleabihf --release -p kobo
```

Run from `kothok/`. The output binary is at:
`kothok/target/armv7-unknown-linux-musleabihf/release/kobo`

### Build the desktop simulator

```powershell
cargo run -p sim
```

## Host LAN IP

Verify before deploying:
```powershell
Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -match "^192\.168\." }
```
Typically `192.168.0.40`.

## Deploy method: FTP (reader NOT running)

Only works when the reader process is not holding the filesystem (fresh boot, before launching from nickel menu, or after reader exits).

```powershell
$ip = "<DEVICE_IP>"
cross build --target armv7-unknown-linux-musleabihf --release -p kobo
$hostMd5 = (Get-FileHash "kothok/target/armv7-unknown-linux-musleabihf/release/kobo" -Algorithm MD5).Hash.ToLower()
curl --ftp-create-dirs -T "kothok/target/armv7-unknown-linux-musleabihf/release/kobo" "ftp://root:root@${ip}/mnt/onboard/.adds/kobo"
```

**FTP verification is NOT trustworthy when the reader IS running.** FTP returns 226 but does not flush to disk. Use HTTP pull + sync instead.

## Deploy method: HTTP pull + sync (reader IS running — the reliable path)

1. Start HTTP server on host:
```powershell
python -m http.server 8099 --directory kothok/target/armv7-unknown-linux-musleabihf/release
```

2. On device via telnet:
```sh
sh /mnt/onboard/.adds/deploy.sh HOST_IP:8099
```

3. Verify on-device:
```sh
md5sum /mnt/onboard/.adds/kobo
```

Must match host build MD5:
```powershell
(Get-FileHash "kothok/target/armv7-unknown-linux-musleabihf/release/kobo" -Algorithm MD5).Hash.ToLower()
```

## Stale binary detection

If MD5 matches but running binary shows old behavior, the launcher is running a different path. On device:
```sh
grep -i bin /mnt/onboard/.adds/run.sh
cat /mnt/onboard/.adds/nm/*
ls -la /tmp/kobo /usr/local/Kobo/kobo 2>/dev/null
```

## Device paths

- Binary: `/mnt/onboard/.adds/kobo`
- Logs: `/mnt/onboard/.adds/kobo.log`
- Error log: `/mnt/onboard/.adds/kobo.err`
- Deploy script: `/mnt/onboard/.adds/deploy.sh`
- Launch script: `/mnt/onboard/.adds/run.sh`
- NickelMenu config: `/mnt/onboard/.adds/nm/config`
- NickelMenu lib: `/usr/local/Kobo/imageformats/libnm.so`

## Line endings

LF only. CRLF breaks the build. Git config:
```
*.rs text eol=lf
*.toml text eol=lf
*.sh text eol=lf
```
