#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
MODEL_FILE="$SCRIPT_DIR/v2x_edca.xml"
QUERY_FILE="$SCRIPT_DIR/v2x_edca.q"
MATRIX_FILE="$SCRIPT_DIR/parameter_matrix.csv"
PROFILE="${1:-edca_v2x}"
VERIFYTA_BIN="${VERIFYTA_BIN:-$(command -v verifyta || true)}"

if [[ -z "$VERIFYTA_BIN" ]]; then
    cat <<'EOF' >&2
verifyta was not found in PATH.

Install UPPAAL and make sure the `verifyta` binary is available, or export:
  VERIFYTA_BIN=/path/to/verifyta

References:
- https://uppaal.org/downloads/
- https://docs.uppaal.org/toolsandapi/verifyta/
EOF
    exit 1
fi

if ! awk -F, -v profile="$PROFILE" 'NR > 1 && $1 == profile { found = 1 } END { exit(found ? 0 : 1) }' "$MATRIX_FILE"; then
    echo "Unknown profile '$PROFILE'. See $MATRIX_FILE for valid entries." >&2
    exit 1
fi

echo "Selected profile:"
awk -F, -v profile="$PROFILE" '
NR == 1 || $1 == profile {
    print
}
' "$MATRIX_FILE"
echo

if [[ "$PROFILE" != "edca_v2x" ]]; then
    cat <<'EOF'
Note:
- The XML model in this directory captures the adaptive `edca_v2x` controller semantics.
- The selected profile is still useful as a checked-in runtime baseline, but it is not a separate timed-automata model in this first iteration.
EOF
    echo
fi

exec "$VERIFYTA_BIN" "$MODEL_FILE" "$QUERY_FILE"
