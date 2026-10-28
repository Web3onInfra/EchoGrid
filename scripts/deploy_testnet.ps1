# Deploy EchoGrid NegaWatt Escrow contract to Stellar Testnet (PowerShell)
param (
    [string]$SourceIdentity = "echogrid_deployer"
)

$ErrorActionPreference = "Stop"
$Network = "testnet"
$RpcUrl = "https://soroban-testnet.stellar.org"
$NetworkPassphrase = "Test SDF Network ; September 2015"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
$ContractDir = Join-Path $RootDir "contracts\negawatt_escrow"
$DeploymentsDir = Join-Path $RootDir "deployments"

Write-Host "=== EchoGrid Testnet Deployment ===" -ForegroundColor Cyan
Write-Host "Identity: $SourceIdentity"
Write-Host "Network: $Network"

$DeployerPubKey = (stellar keys address $SourceIdentity).Trim()
Write-Host "Deployer Public Key: $DeployerPubKey"

Push-Location $ContractDir
try {
    cargo build --target wasm32-unknown-unknown --release
} finally {
    Pop-Location
}

$WasmFile = Join-Path $ContractDir "target\wasm32-unknown-unknown\release\negawatt_escrow.wasm"
$OptimizedWasm = Join-Path $ContractDir "target\wasm32-unknown-unknown\release\negawatt_escrow.optimized.wasm"

stellar contract optimize --wasm $WasmFile --wasm-out $OptimizedWasm

$TokenId = (stellar contract id asset --asset native --network $Network).Trim()
Write-Host "Native Token ID: $TokenId"

$ContractId = (stellar contract deploy --wasm $OptimizedWasm --source $SourceIdentity --network $Network).Trim()
Write-Host "Contract Deployed: $ContractId" -ForegroundColor Green

$BountyStroops = 10000000
stellar contract invoke --id $ContractId --source $SourceIdentity --network $Network -- initialize --municipality $DeployerPubKey --oracle $DeployerPubKey --token $TokenId --bounty $BountyStroops

$DepositStroops = 50000000
stellar contract invoke --id $ContractId --source $SourceIdentity --network $Network -- deposit --amount $DepositStroops

if (-not (Test-Path $DeploymentsDir)) { New-Item -ItemType Directory -Path $DeploymentsDir | Out-Null }
$JsonContent = @"
{
  "network": "$Network",
  "rpc_url": "$RpcUrl",
  "network_passphrase": "$NetworkPassphrase",
  "contract_id": "$ContractId",
  "token_id": "$TokenId",
  "oracle_public_key": "$DeployerPubKey",
  "municipality_public_key": "$DeployerPubKey",
  "bounty_stroops": $BountyStroops,
  "initial_deposit_stroops": $DepositStroops,
  "explorer_url": "https://stellar.expert/explorer/testnet/contract/$ContractId"
}
"@

$JsonPath = Join-Path $DeploymentsDir "testnet.json"
Set-Content -Path $JsonPath -Value $JsonContent -Encoding utf8
Write-Host "=== Deployment Complete ===" -ForegroundColor Green
Write-Host "Saved to: $JsonPath"
