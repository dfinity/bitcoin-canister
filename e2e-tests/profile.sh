#!/usr/bin/env bash
set -Eexuo pipefail
LOG_FILE=$(mktemp)
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
INSTRUCTION_COUNT_FILE="$SCRIPT_DIR"/instructions_count.txt

# Run the syncing test, storing the output into a log file.
bash "$SCRIPT_DIR"/scenario-1.sh 2>&1 | tee "$LOG_FILE"

# Search for the instruction counts in the test and update the instructions count file.
sed -n 's/.*INSTRUCTION COUNT] \(.*\)/\1/p' "$LOG_FILE" > "$INSTRUCTION_COUNT_FILE"
