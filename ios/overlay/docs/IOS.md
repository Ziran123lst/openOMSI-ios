# openOMSI on iOS

openOMSI runs on iPhones and iPads (arm64, iOS 13 or newer; drawing on Metal). It is the
same game as on the computer: the same renderer, simulation, scripts, maps, buses and
mods. The Android build is the template for everything here; only the way content gets
onto the device and the system details differ (no "access to all files" on iOS: the app
lives in its sandbox, and the OMSI 2 folder is imported through the Files app).

- **one app, one window**: the launcher and the game share a window, as on Android
  (`crates/omsi-app/src/ios.rs`, the same `Shell` as `android.rs`). Start is pressed in
  the launcher, the game plays in its window, ending the session (the menu's Quit) goes
  back to the launcher.
- **the launcher for fingers**: the mobile launcher (`launcher/mobile.rs`, `phone.rs`) is
  chosen automatically - `platform::MOBILE` is true for iOS too.
- **on-screen controls** in the game (`crates/omsi-app/src/touch.rs`): exactly the
  Android set - the steering wheel and tilt/touch steering, the pedals, the gearbox, the
  doors, the indicators, the camera, the cab panel (see the table in ANDROID.md). A
  Bluetooth game controller works as on a computer.
- **the content folder** is the app's `Documents/openOMSI` (`$OMSI_CONTENT`); the app's
  settings, profiles and sessions live in `Documents/.openomsi` (`$HOME`). The original
  OMSI 2 installation and the mods are **not** read by path from the whole device - they
  are put into the app's Documents (below).

## Installing the game

An iOS build is not distributed yet: build it yourself (`scripts/build-ios.sh`), or open
`ios/OpenOMSI.xcodeproj` in Xcode and run it on your device (set your signing team).

1. Install and start the app.
2. Put a complete copy of **OMSI 2** (the folder with `Omsi.exe`, `maps` and `Vehicles`)
   into the app's `Documents/openOMSI` - the folder is reachable in the **Files** app
   under *On My iPhone/iPad → openOMSI* (and through iTunes file sharing / Finder), so it
   can be filled by AirDrop, iCloud Drive, a USB cable, a download from the web, ...
   The app must be running once before the folder appears in Files.
3. Start the app, and if it does not find the installation by itself, choose it under
   **Setup → Browse → Use this folder → Save** (the folder, or `Omsi.exe` itself).
4. **Mods**: copy mod folders or `.zip`, `.7z` and `.rar` files into
   `Documents/openOMSI/Mods` (installed when the launcher opens), or install them from
   the launcher's **Mods** page.

> iOS keeps the app in its sandbox: the content must live in the app's own Documents
> folder. An OMSI 2 installation of many gigabytes is imported once and stays on the
> device until the app is deleted (deleting the app deletes the content with it - keep a
> copy on the computer).

The first start uses the phone's lighter graphics defaults (2x MSAA, no ambient
occlusion, a 1024 shadow map, 60 fps, a 900 m object distance), as on Android; when the
frame rate drops below 45 the 3D picture is drawn smaller.

Logs: `game.log` in `Documents/.openomsi` (the launcher's Sessions page shows it), and
`crash.log` in `Documents/openOMSI` when the game stops on an error. `Documents/openOMSI/env.txt`
takes the `OMSI_*` switches a computer takes from its environment, one `NAME=value` a
line (for looking into problems).

## Building

```sh
rustup target add aarch64-apple-ios          # on a Mac with Xcode installed
scripts/build-ios.sh                         # → dist/ios/openOMSI-<version>.app (unsigned)
```

or open `ios/OpenOMSI.xcodeproj` in Xcode, set your team under **Signing &
Capabilities**, and Run. The Xcode project's *Build the Rust game (cargo)* phase builds
`libopenomsi_game.a` (the `omsi-app` library, profile `ios`) for the SDK and
architecture Xcode is building; the simulator builds `aarch64-apple-ios-sim` /
`x86_64-apple-ios` with it.

The `scripts/build-ios.sh` result is unsigned: a stock iPhone refuses it. Sign it in
Xcode (above) or, on a jailbroken device, with `ldid -S`. An App Store release needs the
usual Xcode archive + App Store Connect flow.

## What is and is not there yet

The Rust code is the same game as everywhere else - this is a working port of the whole
engine (wgpu renders through Metal, winit runs `UIApplicationMain`, cpal plays the sound,
gilrs finds Bluetooth controllers). Differences to Android to know about:

- **No tilt steering and no button vibration yet**: they need CoreMotion / UIKit
  bindings (`platform::tilt_steering` returns None on iOS). The steering wheel on the
  screen works.
- **No self-update**: updates arrive from the App Store, not from GitHub
  (`updater::check` is a no-op on iOS; the launcher's Update page only shows the
  version).
- **No Discord status and no file dialogs**: arboard and rfd have no iOS backend, so the
  launcher keeps its own clipboard and `Browse` opens the Files-app import instead of a
  system dialog (the mobile launcher's browser of the content folder).
- **Landscape only** for now (the driving controls are laid out for it; the Info.plist
  declares the two landscape orientations).
- **OpenXR / Steam / DirectInput / HID wheels** are computer-only, as on Android.

The places that differ per platform live in `crates/omsi-app/src/platform.rs` (what a
phone is, tilt, buzz) and in `crates/omsi-app/src/ios.rs` (the app's start); both are
small on purpose.

## Releases

The `.github/workflows/ios-sync.yml` workflow (in this iOS repository) follows the official
releases: when openOMSI publishes a new release, it clones that tag, applies the iOS port
(overlay + numbered `patches/`, and the bundled CJK font), builds the unsigned `.ipa` with
Xcode, and publishes it on the Release `ios-<upstream tag>`. The `.ipa` is unsigned and
installed with LiveContainer (it re-signs with your own certificate). The App Store release
itself is a manual step (signing, archiving, App Store Connect), not in the CI.
