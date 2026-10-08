# openOMSI iOS (unofficial)

An iOS port of [openOMSI](https://github.com/openOMSI-Project/openOMSI) — the Rust
rewrite of the OMSI 2 bus simulator. It renders with Metal.
## Install (no jailbreak)

1. Install **LiveContainer** (SideStore/AltStore), then open the newest
   [**Release**](https://github.com/Ziran123lst/openOMSI-ios/releases) and install the
   `.ipa` from it — LiveContainer re-signs it with your own certificate.
2. Import your OMSI 2 content folder (the one with `Omsi.exe`, `maps`, `Vehicles`) via the
   Files app, then pick it in the app under Setup. See [`docs/IOS.md`](docs/IOS.md).

The `.ipa` is unsigned by design: LiveContainer applies the signature.

## Repository layout

```
ios/
  overlay/        iOS-only files copied over the upstream tree
    crates/omsi-app/src/ios.rs   entry, in-process launch, sandbox, logger, crash hook
    ios/                         Xcode project + main.swift + Info.plist
    scripts/                     build-ios.sh, ios-make-ipa.sh
    docs/IOS.md
  shared.patch    edits to files shared with upstream
  sync.sh         applies the overlay + patch + CJK font onto a fresh upstream checkout
.github/workflows/ios-sync.yml
```

When an upstream change makes `shared.patch` fail, the build is marked failed and the
port needs a manual rebase.

## Build locally (on a Mac)

```sh
git clone --branch <tag> https://github.com/openOMSI-Project/openOMSI.git upstream
cp -R ios upstream/ios && (cd upstream && sh ios/sync.sh)
(cd upstream && OPENOMSI_VERSION=<tag> sh scripts/ios-make-ipa.sh)
```

## Credits / license

openOMSI is by the openOMSI project; this port ships **Noto Sans CJK** (SIL OFL) for
Chinese text because iOS 18 made the system Chinese font private. Same license as the
upstream project.
