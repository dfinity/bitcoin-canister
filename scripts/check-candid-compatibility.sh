#!/usr/bin/env bash
set -euo pipefail

DID_FILES=("canister/candid.did" "watchdog/candid.did")
BASE_REV="${DID_CHECK_REV:-origin/master}"

echo "Checking Candid compatibility against: $BASE_REV"

didc --version

failed=0

for did_file in "${DID_FILES[@]}"; do
    echo "Checking: $did_file"

    # Get the old version from base branch
    tmpfile=$(mktemp)
    if ! git show "$BASE_REV:$did_file" > "$tmpfile" 2>/dev/null; then
        echo "  ⚠️  File is new or doesn't exist in base branch, skipping"
        rm "$tmpfile"
        continue
    fi

    if didc check "$did_file" "$tmpfile"; then
        echo "  $did_file is backward compatible"
    else
        echo "  $did_file has breaking changes!"
        failed=1
    fi

    rm "$tmpfile"
done

if [ $failed -eq 1 ]; then
    echo ""
    echo "❌ Candid compatibility check failed!"
    echo "The new interface has breaking changes that would break existing clients."
    exit 1
fi

echo ""
echo "✅ All Candid interfaces are backward compatible"