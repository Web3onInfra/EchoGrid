# Build and size budget check for PowerShell
$ErrorActionPreference = "Stop"

$MAX_SIZE_BYTES = 65536
$ROOT_DIR = (Get-Item $PSScriptRoot).Parent.FullName

Set-Location $ROOT_DIR

Write-Host "==> Building negawatt_escrow WASM (release, wasm32-unknown-unknown)..."
cargo build --manifest-path contracts/negawatt_escrow/Cargo.toml --target wasm32-unknown-unknown --release

$WASM_RAW = Join-Path $ROOT_DIR "contracts/negawatt_escrow/target/wasm32-unknown-unknown/release/negawatt_escrow.wasm"
$WASM_OPT = Join-Path $ROOT_DIR "contracts/negawatt_escrow/target/wasm32-unknown-unknown/release/negawatt_escrow.optimized.wasm"

Write-Host "==> Optimizing WASM binary..."
if (Get-Command stellar -ErrorAction SilentlyContinue) {
    stellar contract optimize --wasm $WASM_RAW --wasm-out $WASM_OPT
} else {
    Write-Warning "stellar CLI not found; copying unoptimized WASM as fallback"
    Copy-Item $WASM_RAW $WASM_OPT
}

$fileItem = Get-Item $WASM_OPT
$SIZE = $fileItem.Length
$fileHash = (Get-FileHash $WASM_OPT -Algorithm SHA256).Hash.ToLower()

New-Item -ItemType Directory -Force -Path (Join-Path $ROOT_DIR "deployments") | Out-Null
$outFile = Join-Path $ROOT_DIR "deployments/wasm-hash.txt"
"${fileHash}  negawatt_escrow.optimized.wasm  ${SIZE}" | Set-Content -Path $outFile -NoNewline

Write-Host "------------------------------------------------------------"
Write-Host "Artifact:    negawatt_escrow.optimized.wasm"
Write-Host "SHA-256:     $fileHash"
Write-Host "Size:        $SIZE bytes"
Write-Host "Ceiling:     $MAX_SIZE_BYTES bytes"
Write-Host "Output file: deployments/wasm-hash.txt"
Write-Host "------------------------------------------------------------"

if ($SIZE -gt $MAX_SIZE_BYTES) {
    Write-Error "ERROR: Optimized WASM size ($SIZE bytes) exceeds budget ceiling ($MAX_SIZE_BYTES bytes)!"
    exit 1
}

Write-Host "==> Size budget check PASSED ($SIZE <= $MAX_SIZE_BYTES bytes)."
