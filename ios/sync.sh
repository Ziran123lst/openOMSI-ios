#!/bin/sh
# Apply the iOS port onto a fresh upstream checkout (the current working directory):
# copy the iOS-only overlay files, apply the shared-file patches (patches/*.patch, in name
# order), and fetch the bundled CJK font and the patched crc-fast. This lets the port follow
# any upstream release tag without a shared git history.
#
# The patches are made against one upstream release; when upstream moves, their context
# drifts. Strict `git apply` then refuses them, so the script falls back to `patch` with
# fuzz and afterwards verifies that every intended edit is really in the tree (a fuzzed
# hunk can land somewhere unintended). Anything unverified stops the build with the list of
# what is missing, instead of letting it fail later in the compile with a confusing error.
set -eu
HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

fail() {
  echo "::error::iOS port not applied: $*" >&2
  exit 1
}

# 1) iOS-only files (new files the upstream tree does not have)
cp -R "$HERE/overlay/." .

# 2) edits to files shared with upstream (kept as small numbered patches, applied in order)
for p in "$HERE/patches/"*.patch; do
  name="$(basename "$p")"
  if git apply --check --whitespace=nowarn "$p" >/dev/null 2>&1; then
    git apply --whitespace=nowarn "$p"
    continue
  fi

  # The context drifted with the upstream release: try again with fuzz, which lets a hunk
  # land when only its surrounding lines moved. `git apply` has no such tolerance.
  echo "::warning::$name does not apply cleanly to this upstream release; retrying with fuzz"
  patch -p1 --forward --fuzz=3 < "$p" || true
  find . -name '*.orig' -delete
  REJECTED="$(find . -name '*.rej' -print)"
  if [ -n "$REJECTED" ]; then
    echo "$REJECTED" >&2
    for r in $REJECTED; do echo "--- $r" >&2; cat "$r" >&2; done
    find . -name '*.rej' -delete
    fail "$name has hunks that do not apply to this upstream release"
  fi
done

# 2b) every edit the patches promise must be there (file:pattern); see patches/README
MISSING=""
check() {
  # check <file> <fixed string that must occur in it>
  if [ ! -f "$1" ] || ! grep -qF -- "$2" "$1"; then
    MISSING="$MISSING
  - $1 :: $2"
  fi
}
check Cargo.toml '[profile.ios]'
check Cargo.toml 'crc-fast = { path = "vendor/crc-fast" }'
check crates/omsi-app/Cargo.toml "cfg(not(any(target_os = \"android\", target_os = \"ios\")))"
check crates/omsi-launcher-core/Cargo.toml "cfg(not(any(target_os = \"android\", target_os = \"ios\")))"
check crates/omsi-app/src/lib.rs 'mod ios;'
check crates/omsi-app/src/platform.rs 'pub const MOBILE: bool = cfg!(any(target_os = "android", target_os = "ios"));'
check crates/omsi-app/src/startup.rs 'if cfg!(any(target_os = "macos", target_os = "ios")) {'
check crates/omsi-app/src/updater.rs 'updates on iOS are installed from the App Store'
check crates/omsi-app/src/launcher/mod.rs 'crate::ios::boot_log'
check crates/omsi-app/src/launcher/mod.rs 'static FIRST_FRAME'
check crates/omsi-app/src/app.rs '#[cfg(target_os = "ios")]'
check crates/omsi-app/src/app_events.rs 'cfg(not(any(target_os = "android", target_os = "ios")))'
check crates/omsi-app/src/input_script.rs 'cfg(any(target_os = "android", target_os = "ios"))'
check crates/omsi-launcher-core/src/lib.rs 'pub const IN_PROCESS_GAMES: bool = cfg!(any(target_os = "android", target_os = "ios"));'
check crates/omsi-ui/src/text.rs 'const NOTO_CJK'
# crc-fast must come from vendor/, not from crates.io (the .crate ships cdylib crate types)
if grep -A2 '^name = "crc-fast"$' Cargo.lock | grep -q 'registry+https://github.com/rust-lang/crates.io-index'; then
  MISSING="$MISSING
  - Cargo.lock :: crc-fast still resolved from crates.io (its source/checksum lines must go)"
fi
if [ -n "$MISSING" ]; then
  fail "the patches applied, but these edits are missing:$MISSING
Regenerate ios/patches/*.patch against this upstream release."
fi

# 3) the CJK font shipped with the app (kept out of this repo to stay small); since iOS 18
# the system Chinese font is private, so Noto Sans CJK (OFL) is bundled for Chinese text
mkdir -p assets/fonts
if [ ! -s assets/fonts/NotoSansCJKsc-Regular.otf ]; then
  curl -L --retry 3 -o assets/fonts/NotoSansCJKsc-Regular.otf \
    "https://github.com/notofonts/noto-cjk/raw/main/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf"
fi

# 4) patched crc-fast (downloaded from crates.io; its cdylib/staticlib crate types fail the
# iOS cross-build, so keep only "lib")
if [ ! -f vendor/crc-fast/Cargo.toml ]; then
  mkdir -p vendor
  curl -L --retry 3 -o /tmp/crc-fast.crate \
    "https://static.crates.io/crates/crc-fast/crc-fast-1.10.0.crate"
  tar xzf /tmp/crc-fast.crate -C vendor
  rm -rf vendor/crc-fast
  mv vendor/crc-fast-1.10.0 vendor/crc-fast
  # crate-type = ["lib", "cdylib", "staticlib"] -> ["lib"]
  perl -0pi -e 's/crate-type\s*=\s*\[[^\]]*\]/crate-type = ["lib"]/' \
    vendor/crc-fast/Cargo.toml
fi

echo "iOS port applied ($(cat "$HERE/patches/"*.patch | grep -c . || echo 0) shared-file lines)"
