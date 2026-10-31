#!/usr/bin/env bash
set -e

# EchoGrid Demo Recording Preflight Checker
# Verifies all systems are GO before the operator begins screen & audio recording.

ORACLE_URL="${ORACLE_URL:-https://echogrid-oracle.fly.dev}"
APP_URL="${APP_URL:-https://echogrid-pwa.vercel.app}"
TEST_WALLET="${TEST_WALLET:-GA4ZH5HAQ77O2KGZJE2PJ5B4FMKVDGWZUZI7HVLTXPTPCEQGXXLDEHXT}"

echo "=========================================================="
echo "EchoGrid Demo Video Preflight Checker"
echo "=========================================================="
echo "Oracle URL:  $ORACLE_URL"
echo "App URL:     $APP_URL"
echo "Test Wallet: $TEST_WALLET"
echo "----------------------------------------------------------"

ALL_OK=1

# 1. Check Live PWA Reachability
echo -n "[1/4] Checking live PWA reachability... "
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$APP_URL" || echo "000")
if [ "$HTTP_STATUS" = "200" ]; then
  echo "PASS (HTTP 200)"
else
  echo "FAIL (HTTP $HTTP_STATUS)"
  ALL_OK=0
fi

# 2. Check Oracle Readiness & RPC Health
echo -n "[2/4] Checking Oracle /ready endpoint... "
READY_BODY=$(curl -s "$ORACLE_URL/api/v1/ready" || echo "{}")
RPC_HEALTH=$(python3 -c "
import json
try:
    data = json.loads('''$READY_BODY''')
    print('PASS' if data.get('rpc_healthy') is True and data.get('contract_paused') is False else 'FAIL')
except Exception:
    print('FAIL')
" 2>/dev/null || echo "FAIL")

if [ "$RPC_HEALTH" = "PASS" ]; then
  echo "PASS (RPC healthy, contract unpaused)"
else
  echo "FAIL (Readiness response: $READY_BODY)"
  ALL_OK=0
fi

# 3. Check Escrow Contract Configuration & Daily Caps
echo -n "[3/4] Checking Escrow contract caps & config... "
CONFIG_BODY=$(curl -s "$ORACLE_URL/api/v1/config" || echo "{}")
CONFIG_OK=$(python3 -c "
import json
try:
    data = json.loads('''$CONFIG_BODY''')
    bounty = data.get('bounty_stroops', 0)
    network = data.get('network', '')
    print('PASS' if bounty > 0 and network == 'testnet' else 'FAIL')
except Exception:
    print('FAIL')
" 2>/dev/null || echo "FAIL")

if [ "$CONFIG_OK" = "PASS" ]; then
  echo "PASS (Bounty active, testnet verified)"
else
  echo "FAIL (Config response: $CONFIG_BODY)"
  ALL_OK=0
fi

# 4. Check Demo Wallet Challenge Issuance & Cooldown Status
echo -n "[4/4] Checking Challenge issuance & cooldown status... "
CHALLENGE_RESP=$(curl -s -w "\n%{http_code}" "$ORACLE_URL/api/v1/challenge?wallet=$TEST_WALLET" || echo -e "{}\n000")
HTTP_CODE=$(echo "$CHALLENGE_RESP" | tail -n 1)
CHALLENGE_JSON=$(echo "$CHALLENGE_RESP" | head -n -1)

if [ "$HTTP_CODE" = "200" ]; then
  echo "PASS (Challenge nonce issued successfully)"
elif [ "$HTTP_CODE" = "429" ]; then
  echo "WARN (Wallet is currently in cooldown; wait for cooldown to expire before recording claim)"
else
  echo "FAIL (HTTP $HTTP_CODE: $CHALLENGE_JSON)"
  ALL_OK=0
fi

echo "----------------------------------------------------------"
if [ "$ALL_OK" -eq 1 ]; then
  echo "RESULT: PREFLIGHT GREEN. All systems ready for demo recording."
  exit 0
else
  echo "RESULT: PREFLIGHT FAILED. Address the errors above before recording."
  exit 1
fi
