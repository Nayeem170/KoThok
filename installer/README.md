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

## Uninstall

### Windows

1. Download `uninstall.bat` and `uninstall.ps1`
2. Plug in your Kobo via USB
3. Double-click `uninstall.bat`
4. Type `yes` to confirm
5. Eject, unplug, and reboot the Kobo (hold power 15s, release, press again)

### macOS

1. Download `uninstall.command` and `uninstall.ps1`
2. Plug in your Kobo via USB
3. Double-click `uninstall.command` in Finder
4. Type `yes` to confirm
5. Eject, unplug, and reboot the Kobo (hold power 15s, release, press again)

### Linux

1. Download `uninstall.sh` and `uninstall.ps1`
2. Plug in your Kobo via USB
3. Run: `chmod +x uninstall.sh && ./uninstall.sh`
4. Type `yes` to confirm
5. Eject, unplug, and reboot the Kobo (hold power 15s, release, press again)

After the reboot the "KoThok" button is gone from the device menu.

## What it does

The installer downloads KoThok from GitHub and copies it to your Kobo. It also
adds a "KoThok" button to the device's home menu.

There are two flows:

- **First install** - downloads an extra package (`KoboRoot.tgz`), copies it to
  the Kobo, and asks you to eject and reboot. The device shows an "Updating..."
  screen for about 30 seconds. After the reboot the menu button appears.
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

There is also a built-in failsafe: if you power the Kobo off within 20 seconds
of turning it on, NickelMenu uninstalls itself automatically.

After either method the hamburger menu is back to stock.

Reference: [NickelMenu documentation](https://github.com/pgaskin/NickelMenu/blob/master/res/doc).
