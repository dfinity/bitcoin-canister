#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

pushd "$SCRIPT_DIR"

# Build the benchmarks canister
bash ../scripts/build-canister.sh benchmarks

# Run the benchmarks, decode the output.
drun ./drun.txt --instruction-limit 99999999999999 # \
#    | awk '{ print $3 }' \
 #   | grep "44.*" -o \
  #  | xargs -L 1 didc decode
