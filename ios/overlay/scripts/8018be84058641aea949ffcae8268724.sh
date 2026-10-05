#!/bin/sh
# Build the unsigned iOS .ipa of openOMSI, ready for LiveContainer (which re-signs it on
# install): the Rust game as a static library, Xcode links it into the .app, then the
# .app is wrapped in Payload/ and zipped into an .ipa. Runs on a Mac with Xcode.
#
#   scripts/ios-make-ipa.sh                -> dist/ios/openOMSI-<version>.ipa
#
# The .ipa is unsigned on purpose: LiveContainer applies the user's own certificate.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
version="${OPENOMSI_VERSION:-$(sh scripts/version.sh 2>/dev/null || echo 0.0.0)}"
export OPENOMSI_VERSION="$version"

# 1) cargo staticlib + xcodebuild .app (this script also removes and rebuilds dist/ios)
sh scripts/build-ios.sh

app="dist/ios/openOMSI-$version.app"
[ -d "$app" ] || { echo "expected .app was not built: $app" >&2; exit 1; }

# 2) wrap the .app as Payload/OpenOMSI.app and zip it into an .ipa
stage="dist/ios/ipa-stage"
rm -rf "$stage"
mkdir -p "$stage/Payload"
cp -R "$app" "$stage/Payload/OpenOMSI.app"
ipa="dist/ios/openOMSI-$version-ios-arm64.ipa"
rm -f "$ipa"
(cd "$stage" && zip -qry "../openOMSI-$version-ios-arm64.ipa" Payload)
rm -rf "$stage"

echo
echo "Built $PWD/$ipa (unsigned; LiveContainer re-signs it)"
