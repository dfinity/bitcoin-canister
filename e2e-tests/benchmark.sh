#!/usr/bin/env bash
set -Eexuo pipefail

BENCH_OUTPUT=$(cargo bench)

set +e
REGRESSIONS=$( echo "$BENCH_OUTPUT" |  grep -c "regressed by" )
set -e

if [[ $REGRESSIONS != 0 ]]; then
  echo "FAIL! Performance regressions are detected. 
        Make sure that you results.yml represent results
        of benchmarking current master branch."
  exit 1
fi

echo "SUCCESS! Performance regressions are not detected."
