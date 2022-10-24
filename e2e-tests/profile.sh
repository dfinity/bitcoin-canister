#!/usr/bin/env bash
set -Eexuo pipefail
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

# Run scenario 1, searching for instruction counts in the logs and outputting them to a file.
LOG_FILE=$(mktemp)
INSTRUCTION_COUNT_FILE="$SCRIPT_DIR"/instructions_count.txt
bash "$SCRIPT_DIR"/scenario-1.sh 2>&1 | tee "$LOG_FILE"
sed -n 's/.*INSTRUCTION COUNT] \(.*\)/\1/p' "$LOG_FILE" > "$INSTRUCTION_COUNT_FILE"

# Run scenario 2, searching for instruction counts in the logs and outputting them to a file.
LOG_FILE=$(mktemp)
INSTRUCTION_COUNT_FILE="$SCRIPT_DIR"/profiling/scenario-2.txt
bash "$SCRIPT_DIR"/scenario-2.sh 2>&1 | tee "$LOG_FILE"
sed -n 's/.*INSTRUCTION COUNT] \(.*\)/\1/p' "$LOG_FILE" > "$INSTRUCTION_COUNT_FILE"
