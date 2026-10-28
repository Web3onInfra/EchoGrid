#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/.cargo/bin:$PATH"

# Size ceiling budget in bytes (documented ceiling: 65,536 bytes / 64 KB)
MAX_SIZE_BYTES=65536

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "==> Building negawatt_escrow WASM (release, wasm32-unknown-unknown)..."
cargo build --manifest-path contracts/negawatt_escrow/Cargo.toml --target wasm32-unknown-unknown --release

WASM_RAW="contracts/negawatt_escrow/target/wasm32-unknown-unknown/release/negawatt_escrow.wasm"
WASM_OPT="contracts/negawatt_escrow/target/wasm32-unknown-unknown/release/negawatt_escrow.optimized.wasm"

echo "==> Optimizing WASM binary..."
if command -v stellar &> /dev/null; then
    stellar contract optimize --wasm "$WASM_RAW" --wasm-out "$WASM_OPT"
else
    echo "WARNING: stellar CLI not found; copying unoptimized WASM as fallback"
    cp "$WASM_RAW" "$WASM_OPT"
fi

# Size computation in bytes
if stat -c %s "$WASM_OPT" &> /dev/null; then
    SIZE=$(stat -c %s "$WASM_OPT")
else
    SIZE=$(wc -c < "$WASM_OPT" | tr -d ' ')
fi

# SHA-256 calculation
if command -v sha256sum &> /dev/null; then
    HASH=$(sha256sum "$WASM_OPT" | awk '{print $1}')
elif command -v shasum &> /dev/null; then
    HASH=$(shasum -a 256 "$WASM_OPT" | awk '{print $1}')
else
    HASH=$(openssl dgst -sha256 "$WASM_OPT" | awk '{print $NF}')
fi

mkdir -p deployments
echo "${HASH}  negawatt_escrow.optimized.wasm  ${SIZE}" > deployments/wasm-hash.txt

echo "------------------------------------------------------------"
echo "Artifact:    negawatt_escrow.optimized.wasm"
echo "SHA-256:     ${HASH}"
echo "Size:        ${SIZE} bytes"
echo "Ceiling:     ${MAX_SIZE_BYTES} bytes"
echo "Output file: deployments/wasm-hash.txt"
echo "------------------------------------------------------------"

if [ "$SIZE" -gt "$MAX_SIZE_BYTES" ]; then
    echo "ERROR: Optimized WASM size (${SIZE} bytes) exceeds budget ceiling (${MAX_SIZE_BYTES} bytes)!" >&2
    exit 1
fi

echo "==> Size budget check PASSED (${SIZE} <= ${MAX_SIZE_BYTES} bytes)."
