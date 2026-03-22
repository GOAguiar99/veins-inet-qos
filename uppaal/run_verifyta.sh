#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
MATRIX_FILE="$SCRIPT_DIR/parameter_matrix.csv"
ENV_FILE="${UPPAAL_ENV_FILE:-$SCRIPT_DIR/.env}"
PROFILE="${1:-edca_v2x}"

if [[ -f "$ENV_FILE" ]]; then
    # shellcheck disable=SC1090
    source "$ENV_FILE"
fi

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

MODEL_BASENAME="$(awk -F, -v profile="$PROFILE" '
NR == 1 {
    for (i = 1; i <= NF; i++) {
        if ($i == "model_file")
            modelCol = i;
    }
}
NR > 1 && $1 == profile {
    if (modelCol > 0 && modelCol <= NF)
        print $modelCol;
    else
        print "v2x_edca.xml";
    exit;
}
' "$MATRIX_FILE")"

if [[ -z "$MODEL_BASENAME" ]]; then
    MODEL_BASENAME="v2x_edca.xml"
fi

MODEL_FILE="$SCRIPT_DIR/$MODEL_BASENAME"
if [[ ! -f "$MODEL_FILE" ]]; then
    echo "Model file '$MODEL_FILE' not found for profile '$PROFILE'." >&2
    exit 1
fi

echo "Selected profile:"
awk -F, -v profile="$PROFILE" '
NR == 1 || $1 == profile {
    print
}
' "$MATRIX_FILE"
echo

if [[ "$PROFILE" == "plain" || "$PROFILE" == "edca_only" ]]; then
    cat <<'EOF'
Note:
- This profile is a runtime baseline reference.
- Formal checks still use the adaptive timed-automata model selected for this row.
EOF
    echo
fi

if [[ -n "${UPPAAL_LICENSE_KEY:-}" ]]; then
    export UPPAAL_LICENSE_KEY
fi

exec "$VERIFYTA_BIN" "$MODEL_FILE"
