#!/bin/sh
# Build openOMSI for iOS as dist/ios/openOMSI-<version>.app (and a zip of it beside):
# the game as a static library, the Xcode project of `ios/` around it. Needs a Mac with
# Xcode (the iOS SDK), Rust with the `aarch64-apple-ios` target
# (`rustup target add aarch64-apple-ios`) and, to install on a device, a signing identity -
# the easiest way is to open `ios/OpenOMSI.xcodeproj` in Xcode, choose your team under
# Signing & Capabilities, and run.
#
#   scripts/build-ios.sh            build the .app (unsigned) into dist/ios/
#
# The .app is unsigned here: a stock iPhone refuses it. To run it on your own device,
# either sign it in Xcode (above) or, on a jailbroken device, `ldid -S` it. The App Store
# needs the full Xcode archive + App Store Connect flow.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
export PATH="$HOME/.cargo/bin:$PATH"
version="${OPENOMSI_VERSION:-$(sh scripts/version.sh 2>/dev/null || echo 0.0.0)}"
export OPENOMSI_VERSION="$version"
target=aarch64-apple-ios
build=build/ios
rm -rf "$build" dist/ios
mkdir -p "$build" dist/ios

# --- the native code (a static library; the Xcode project links it with -lopenomsi_game)
# (`cargo rustc --crate-type staticlib`, as Android's script does with cdylib: the crate's
# [lib] declares no crate-type, so the plain build produces only an rlib)
cargo rustc --locked --profile ios --target "$target" -p omsi-app --lib --crate-type staticlib

# --- the app (the Xcode project runs cargo again for the SDK/arch it builds)
xcodebuild -project ios/OpenOMSI.xcodeproj -scheme openOMSI -configuration Release \
    -sdk iphoneos -derivedDataPath "$build/DerivedData" \
    CODE_SIGNING_ALLOWED=NO build
app="$(find "$build/DerivedData/Build/Products/Release-iphoneos" -maxdepth 1 -name '*.app' | head -1)"
[ -n "$app" ] || { echo "no .app was built" >&2; exit 1; }
bundle="dist/ios/openOMSI-$version.app"
cp -R "$app" "$bundle"
(cd dist/ios && zip -qry "openOMSI-$version-ios-arm64.zip" "openOMSI-$version.app")

cat <<EOF

openOMSI $version for iOS: $PWD/$bundle (unsigned)
  The zip: $PWD/dist/ios/openOMSI-$version-ios-arm64.zip

The .app is not signed: a stock iPhone will not install it. To sign it, open
ios/OpenOMSI.xcodeproj in Xcode, set your team under Signing & Capabilities, and run;
or, on a jailbroken device, sign it with ldid. The OMSI 2 content is imported through
the Files app (see docs/IOS.md).
EOF
