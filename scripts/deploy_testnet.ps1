# EchoGrid: Testnet Deploy Script (PowerShell wrapper)
param(
    [switch]$DryRun
)

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RootDir = Split-Path -Parent $ScriptDir
Set-Location $RootDir

$argsList = @()
if ($DryRun) {
    $argsList += "--dry-run"
}

bash scripts/deploy_testnet.sh @argsList
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
