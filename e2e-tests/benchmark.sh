#!/usr/bin/env bash
set -Eexuo pipefail

set +e
REGRESSIONS=$(cargo bench | grep -c "regressed by")
set -e

if [[ $REGRESSIONS != 0 ]]; then
  echo "FAIL! Performance regressions are detected. 
        Make sure that you results.yml represent results
        of benchmarking current master branch with drun
        \"release-2023-09-27_23-01\"."
  exit 1
fi

echo "SUCCESS! Performance regressions are not detected."
