# KoThok Installer

## Install

### Windows

1. Download `install.bat` and `install.ps1`
2. Plug in your Kobo via USB
3. Double-click `install.bat`
4. Follow the on-screen steps

### macOS

1. Download `install.command` and `install.ps1`
2. Plug in your Kobo via USB
3. Double-click `install.command` in Finder
4. Follow the on-screen steps

### Linux

1. Download `install.sh` and `install.ps1`
2. Plug in your Kobo via USB
3. Run: `chmod +x install.sh && ./install.sh`
4. Follow the on-screen steps

## No script? Manual install

Don't want to install PowerShell 7, or just don't want to run a script
at all? You can install and update KoThok by copying files to the Kobo's
USB drive - no scripts, no command line.

### First-time install (one-time)

Download `KoThok-<version>-manual-install.zip` from
[Releases](https://github.com/Nayeem170/KoThok/releases).

1. Unzip it - you'll get `KoboRoot.tgz` and `INSTRUCTIONS.txt`.
2. Plug in your Kobo via USB.
3. Copy `KoboRoot.tgz` into the Kobo's `.kobo` folder (a hidden folder next to
   `.adds` - turn on "show hidden files" in your file browser if you don't see
   it).
4. Eject, unplug, then reboot the Kobo (power it off and back on).
5. Watch for an "Updating..." screen (~30s) - this installs NickelMenu and
   KoThok in one step, then it restarts itself. Don't unplug or power off
   during it.
6. Once booted, open the hamburger menu and tap **KoThok**.

Fonts and a sample book are bundled in, same as the scripted install.

### Update (every time after that)

Updates do not need a reboot. Just copy the new binary over the old one:

1. Download `kothok` (the raw binary) from the latest
   [Release](https://github.com/Nayeem170/KoThok/releases).
2. Plug in your Kobo via USB.
3. Open the `.adds` folder on the Kobo USB drive.
4. Copy the `kothok` binary into `.adds/`, overwriting the existing `kothok`
   file (it sits directly in `.adds/`, not in a subfolder).
5. Eject, unplug, and launch KoThok from the hamburger menu.

That's it - the next launch runs the new version.

## Uninstall

### Windows

1. Download `uninstall.bat` and `uninstall.ps1`
2. Plug in your Kobo via USB
3. Double-click `uninstall.bat`
4. Type `yes` to confirm
5. Eject, unplug, and reboot the Kobo (power it off and back on)

### macOS

1. Download `uninstall.command` and `uninstall.ps1`
2. Plug in your Kobo via USB
3. Double-click `uninstall.command` in Finder
4. Type `yes` to confirm
5. Eject, unplug, and reboot the Kobo (power it off and back on)

### Linux

1. Download `uninstall.sh` and `uninstall.ps1`
2. Plug in your Kobo via USB
3. Run: `chmod +x uninstall.sh && ./uninstall.sh`
4. Type `yes` to confirm
5. Eject, unplug, and reboot the Kobo (power it off and back on)

After the reboot the "KoThok" button is gone from the device menu.

## What it does

The installer downloads KoThok from GitHub and copies it to your Kobo. It also
adds a "KoThok" button to the device's home menu.

There are two flows:

- **First install** - downloads an extra package (`KoboRoot.tgz`), copies it to
  the Kobo's `.kobo` folder, and asks you to eject and reboot. The device shows
  an "Updating..." screen for about 30 seconds. After the reboot the menu button
  appears.
- **Update** - just copies the new binary in place. No reboot, no extra package.

The uninstaller removes KoThok and **only** its own menu entry. Your book, your
reading position, and other mods' menu entries stay on the device.

**Requirements:**

- [PowerShell 7](https://learn.microsoft.com/powershell/scripting/install/installing-powershell)
  (`pwsh`). Windows ships with PowerShell 5.1, which is not enough - install
  PowerShell 7 first. Needed for both install and uninstall.
- Internet connection (install only; uninstall works offline).

## NickelMenu (the menu button system)

KoThok uses NickelMenu to add its button to the device's hamburger menu.
NickelMenu is shared - other mods like KOReader and Plato also use it. The
uninstaller removes KoThok's entry but leaves NickelMenu itself in place so
those other mods keep working.

If you want NickelMenu fully gone (you removed every mod that uses it):

1. Create an empty file named `uninstall` inside the `.adds/nm/` folder on the
   Kobo USB drive. This is a normal USB file operation - no telnet needed.
2. Eject and reboot the Kobo. NickelMenu detects the `uninstall` file on boot
   and removes itself cleanly, including its library file inside the device.

If the hamburger menu ever disappears, just run the installer again. It
reinstalls everything in one step.

After either method the hamburger menu is back to stock.

Reference: [NickelMenu documentation](https://github.com/pgaskin/NickelMenu/blob/master/res/doc).

## Logs and crash reports

When you plug the Kobo into a computer over USB, these files are visible in the
`.adds` folder:

| File | Contents |
|------|----------|
| `.adds/kothok.log` | Application log (startup, BT, WiFi, audio events) |
| `.adds/crash.log` | Crash traces if KoThok panicked or crashed |
| `.adds/kothok.err` | stderr output (background service messages) |

If KoThok crashes or behaves unexpectedly, copy `crash.log` and `kothok.log`
and share them with the developer. These files reset on each launch of KoThok,
so copy them before relaunching.
