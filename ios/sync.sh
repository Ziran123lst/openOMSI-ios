#!/bin/sh
# Apply the iOS port onto a fresh upstream checkout (the current working directory):
# copy the iOS-only overlay files, apply the shared-file patches (patches/*.patch, in name
# order), and fetch the bundled CJK font and the patched crc-fast. This lets the port follow
# any upstream release tag without a shared git history.
set -eu
HERE="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"

# 1) iOS-only files (new files the upstream tree does not have)
cp -R "$HERE/overlay/." .

# 2) edits to files shared with upstream (kept as small numbered patches, applied in order)
for p in "$HERE/patches/"*.patch; do
  if [ -d .git ]; then
    git apply --whitespace=nowarn "$p"
  else
    patch -p1 --forward < "$p"
  fi
done

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
