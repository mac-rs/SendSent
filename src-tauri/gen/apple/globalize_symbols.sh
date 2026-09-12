#!/bin/sh
# Xcode 27's SwiftPM internalizes @_cdecl exports in static Swift products, so
# the Rust static library ends up referencing symbols that are local (lowercase
# 't') and the app link fails with "Undefined symbols".
#
# swift-rs 1.0.8 attempts to globalize them but misses symbols that come from
# SwiftPM dependency members (e.g. swift-rs' own `SwiftRs` package:
# `_release_object`, `_retain_object`, `_string_from_bytes`).
#
# Run this right after the Rust static library is built (before the app links)
# to promote exactly the local symbols that the archive still references.
set -eu

CONFIGURATION="${1:-Release}"

LIB="$SRCROOT/Externals/arm64/$CONFIGURATION/libapp.a"
[ -f "$LIB" ] || LIB="$SRCROOT/Externals/x86_64/$CONFIGURATION/libapp.a"
[ -f "$LIB" ] || exit 0

RUSTC="$(command -v rustc || true)"
[ -n "$RUSTC" ] || exit 0
HOST="$("$RUSTC" -vV | sed -n 's/^host: //p')"
OBJCOPY="$("$RUSTC" --print sysroot)/lib/rustlib/$HOST/bin/llvm-objcopy"
if [ ! -x "$OBJCOPY" ]; then
  echo "globalize_symbols: llvm-objcopy not found (rustup component add llvm-tools); skipping" >&2
  exit 0
fi

FLAGS="$(nm "$LIB" 2>/dev/null | awk '
  NF == 2 && $1 == "U" { undef[$2] = 1; next }
  NF >= 3 && $2 == "t" {
    n = $3
    if (n ~ /^_[A-Za-z0-9_]+$/ && n !~ /block_copy_helper/ && n !~ /block_destroy_helper/) {
      def[n] = 1
    }
  }
  END { for (s in def) if (s in undef) printf " --globalize-symbol=%s", s }
')"
[ -n "$FLAGS" ] || exit 0

TMP="$LIB.gtmp"
# shellcheck disable=SC2086
"$OBJCOPY" $FLAGS "$LIB" "$TMP"
mv "$TMP" "$LIB"
echo "globalize_symbols: promoted internalized @_cdecl symbols in $LIB"
