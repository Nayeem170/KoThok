---
name: kobo-device
description: Kobo Libra Colour device management — telnet, sysfs paths, frontlight, battery, wifi, BT, logs, nickel, wakelocks, crash diagnostics
---

# kobo-device

Interact with a Kobo Libra Colour device (MT8110 SoC, Kaleido 3 color e-ink) via telnet/SSH and sysfs.

## Connecting

```powershell
telnet <DEVICE_IP>
```
Credentials: `root` / no password.

IP changes on every reboot (DHCP). Find it:
```powershell
Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.IPAddress -match "^192\.168\." }
```

## Device sysfs paths

| Resource | Path |
|----------|------|
| Frontlight (main) | `/sys/class/backlight/lm3630a_led/brightness` (max=100) |
| Frontlight color A | `/sys/class/backlight/lm3630a_leda/brightness` |
| Frontlight color B | `/sys/class/backlight/lm3630a_ledb/brightness` |
| Blank/unblank | `/sys/class/graphics/fb0/bl_power` (0=unblank, 4=powerdown) |
| Battery | `/sys/class/power_supply/mc13892_battery/` |
| Battery status | `cat /sys/class/power_supply/mc13892_battery/status` |
| Battery capacity | `cat /sys/class/power_supply/mc13892_battery/capacity` |
| Wakelock (dev_probe) | `/sys/power/wakelocks/dev_probe` |
| WiFi interface | `wlan0` |
| BT service | `btservice` |

## Frontlight notes

- Use `lm3630a_led` only (max_brightness=100).
- `lm3630a_leda` and `lm3630a_ledb` are color channels, NOT the main frontlight.
- After extended sleep (>5 min), the I2C bus may be idle. Toggle `bl_power` (4→0) to force driver reinit before restoring brightness.

## Battery

```sh
cat /sys/class/power_supply/mc13892_battery/status
cat /sys/class/power_supply/mc13892_battery/capacity
```

## Nickel (Kobo firmware UI)

- `run.sh` kills: `nickel nickel.orig hindenburg sickel fickel strickel fontickel foxitpdf iink fmon`
- `run.sh` keeps alive: `adobehost btservice mtkbtd`
- Exit to nickel requires reboot (SIGSTOP/SIGCONT corrupts Qt Embedded QWS GUI state).
- NickelMenu `libnm.so` at `/usr/local/Kobo/imageformats/libnm.so`, config at `/mnt/onboard/.adds/nm/config`.

## Logs

```sh
cat /mnt/onboard/.adds/kobo.log
cat /mnt/onboard/.adds/kobo.err
```

## Launching the reader

From nickel menu (NickelMenu "Book Reader" entry) or:
```sh
sh /mnt/onboard/.adds/run.sh
```
`run.sh` kills nickel stack, launches reader, reboots on exit.

## Deploying while reader is running

```sh
sh /mnt/onboard/.adds/deploy.sh HOST_IP:PORT
```

## Crash diagnostics

The release profile is `panic="abort"`. A panic = SIGABRT = process death = run.sh reboots.
Check crash log: `cat /mnt/onboard/.adds/crash.log` (if panic hook wrote one).

## WiFi

```sh
ifconfig wlan0 up
wpa_supplicant -B -i wlan0 -c /etc/wpa_supplicant/wpa_supplicant.conf
dhcpcd wlan0
```

## Bluetooth A2DP

- A2DP sink via `btservice`.
- After multiple connect/disconnect cycles the BT stack gets confused. Fix: re-pair from device Bluetooth settings.
- A2DP HAL: `/system/lib/hw/audio.a2dp.default.so` (AOSP).

## Reboot

```sh
reboot
```

## Process management

```sh
ps aux | grep -E "nickel|kobo"
killall -TERM nickel
killall -KILL nickel
```
