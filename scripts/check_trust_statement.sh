#!/usr/bin/env bash
set -e

# EchoGrid Canonical Trust Statement Drift Checker
# Verifies that every file listed in docs/trust-model.md contains the canonical text verbatim.

TRUST_MODEL_FILE="docs/trust-model.md"

if [ ! -f "$TRUST_MODEL_FILE" ]; then
  echo "Error: Trust model file '$TRUST_MODEL_FILE' not found."
  exit 1
fi

# Extract canonical text between markers
CANONICAL_TEXT=$(python3 -c "
with open('$TRUST_MODEL_FILE', 'r', encoding='utf-8') as f:
    content = f.read()
start_marker = '<!-- CANONICAL_TRUST_STATEMENT_START -->'
end_marker = '<!-- CANONICAL_TRUST_STATEMENT_END -->'
if start_marker not in content or end_marker not in content:
    raise ValueError('Start or end marker not found in $TRUST_MODEL_FILE')
start = content.index(start_marker) + len(start_marker)
end = content.index(end_marker)
text = ' '.join(content[start:end].split())
print(text)
")

if [ -z "$CANONICAL_TEXT" ]; then
  echo "Error: Failed to extract canonical trust statement from $TRUST_MODEL_FILE."
  exit 1
fi

echo "=========================================================="
echo "EchoGrid Canonical Trust Statement Checker"
echo "=========================================================="
echo "Canonical Statement:"
echo "\"$CANONICAL_TEXT\""
echo "----------------------------------------------------------"

# Extract targets from front-matter
TARGETS=$(python3 -c "
with open('$TRUST_MODEL_FILE', 'r', encoding='utf-8') as f:
    lines = f.readlines()
in_targets = False
for line in lines:
    line = line.strip()
    if line.startswith('canonical_targets:'):
        in_targets = True
        continue
    if in_targets:
        if line.startswith('- '):
            print(line[2:].strip().strip('\"').strip('\''))
        elif line and not line.startswith('#'):
            break
")

FAILED=0
CHECKED=0

for TARGET in $TARGETS; do
  if [ ! -f "$TARGET" ]; then
    echo "[SKIPPED] $TARGET (file not yet created in this branch/phase)"
    continue
  fi

  CHECKED=$((CHECKED + 1))
  MATCH=$(python3 -c "
with open('$TARGET', 'r', encoding='utf-8', errors='ignore') as f:
    content = ' '.join(f.read().split())
canonical = '''$CANONICAL_TEXT'''
if canonical in content:
    print('MATCH')
else:
    print('DRIFT')
")

  if [ "$MATCH" = "MATCH" ]; then
    echo "[PASS] $TARGET contains canonical trust statement verbatim."
  else
    echo "[FAIL] $TARGET has drifted from the canonical trust statement!"
    FAILED=$((FAILED + 1))
  fi
done

echo "----------------------------------------------------------"
echo "Checked $CHECKED existing target file(s)."

if [ $FAILED -gt 0 ]; then
  echo "Error: Trust statement drift detected in $FAILED file(s)."
  exit 1
fi

echo "All existing targets contain the canonical statement verbatim."
exit 0
