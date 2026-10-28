# NegaWatt Escrow: Reproducible Build & Size Budget

## 1. Toolchain Specifications

This smart contract compiles to WebAssembly (`wasm32-unknown-unknown`) using the Rust toolchain and Soroban SDK:

| Component | Verified Version | Notes |
| :--- | :--- | :--- |
| **Rust Toolchain** | `1.84+` / `1.85+` (stable) | `x86_64-pc-windows-gnu` / `x86_64-unknown-linux-gnu` |
| **Compilation Target** | `wasm32-unknown-unknown` | Added via `rustup target add wasm32-unknown-unknown` |
| **Stellar CLI** | `28.1.0` | CLI binary with `stellar-xdr 28.0.0` |
| **Soroban SDK** | `21.0.0` (SDK v21.7.7 host) | Profile `opt-level = "z"`, `overflow-checks = true` |
| **Pinned Dependencies** | `ed25519-dalek = 2.2.0`, `curve25519-dalek = 4.1.3` | Pinned in `Cargo.lock` to prevent compiler incompatibilities |

> [!NOTE]
> Recent Stellar CLI releases default `--target=wasm32v1-none`. Because Soroban SDK v21 with `alloc` imports `std` stubs during compilation, this contract targets `wasm32-unknown-unknown` directly for reproducible builds.

---

## 2. Size Budget & Ceiling

Stellar validators meter WASM bytecode size and rent costs. To ensure fast propagation and minimal gas overhead, EchoGrid enforces a strict size ceiling:

- **Maximum Size Budget Ceiling**: `65,536 bytes` (64 KB)
- **Current Raw Release Size**: `32,727 bytes` (~32.7 KB)
- **Current Optimized Size**: `25,455 bytes` (~25.4 KB)
- **Budget Headroom**: ~61.1% headroom remaining under the ceiling

Any build exceeding `65,536 bytes` fails CI immediately.

---

## 3. Reproducible Build Instructions

### One-Step Automated Build
Run the build script from the repository root:

```bash
# Linux / macOS / Git Bash
bash scripts/build_contract.sh

# Windows PowerShell
powershell -ExecutionPolicy Bypass -File scripts/build_contract.ps1
```

### Manual Step-by-Step Build

1. **Compile WASM with release profile**:
   ```bash
   cargo build --manifest-path contracts/negawatt_escrow/Cargo.toml --target wasm32-unknown-unknown --release
   ```

2. **Optimize WASM using Stellar CLI**:
   ```bash
   stellar contract optimize \
     --wasm contracts/negawatt_escrow/target/wasm32-unknown-unknown/release/negawatt_escrow.wasm \
     --wasm-out contracts/negawatt_escrow/target/wasm32-unknown-unknown/release/negawatt_escrow.optimized.wasm
   ```

3. **Verify SHA-256 and size**:
   ```bash
   sha256sum contracts/negawatt_escrow/target/wasm32-unknown-unknown/release/negawatt_escrow.optimized.wasm
   ```

The canonical hash and size are saved in `deployments/wasm-hash.txt`.
