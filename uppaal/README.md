# UPPAAL Workflow

This directory contains the offline formal model for the adaptive V2X EDCA policy implemented in `veins_qos`.

## Purpose

- keep OMNeT++ runtime behavior unchanged
- model-check the current `V2xHcf` + `V2xEdcaFsmController` policy offline
- validate timing/logic assumptions before (or alongside) simulation sweeps

UPPAAL here is verification support, not a runtime MAC replacement.

## Files

- `v2x_edca.xml`: timed automata model (with embedded verification queries)
- `v2x_edca_fast.xml`: tiny finite-state timed automata model for fast sanity checks
- `v2x_edca.q`: query mirror/reference text
- `MODEL_GUIDE.md`: reading guide
- `parameter_matrix.csv`: mapping between OMNeT++ configs and formal parameters
- `run_verifyta.sh`: helper runner

## Run

```bash
cd /home/goaguiar/master/master_veins
./uppaal/run_verifyta.sh
```

Optional profile argument:

```bash
./uppaal/run_verifyta.sh edca_v2x
./uppaal/run_verifyta.sh edca_v2x_fast
```

Optional license key setup:

```bash
cp uppaal/.env.example uppaal/.env
# edit uppaal/.env and set UPPAAL_LICENSE_KEY only if your local verifyta setup needs it
```

## Important Clarifications

- You do not need OMNeT++ running to execute UPPAAL checks.
- `verifyta` must be installed separately and reachable in `PATH`, or set:
  - `VERIFYTA_BIN=/absolute/path/to/verifyta`
- The default XML model corresponds to the `edca_v2x` policy semantics.
- `plain` and `edca_only` in `parameter_matrix.csv` are runtime baselines for comparison context, not separate automata models.

## Common Errors

- `verifyta: command not found`
  - install UPPAAL CLI tools or export `VERIFYTA_BIN`
- `EXCEPTION: Unexpected end.`
  - usually indicates license/setup issue with the `verifyta` binary invocation
