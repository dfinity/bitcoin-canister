#!/usr/bin/env bash
set -euo pipefail

# Configuration
DID_FILES=("canister/candid.did" "watchdog/candid.did")
BASE_REV="${DID_CHECK_REV:-origin/master}"

echo "Checking Candid compatibility against: $BASE_REV"

if ! command -v didc &> /dev/null; then
    echo "Installing didc..."
    DIDC_VERSION="2025-10-16"
    OS=$(uname -s)
    ARCH=$(uname -m)

    if [ "$OS" = "Darwin" ]; then
        DIDC_BINARY="didc-macos"
    elif [ "$OS" = "Linux" ]; then
        if [ "$ARCH" = "armv7l" ] || [ "$ARCH" = "armv6l" ]; then
            DIDC_BINARY="didc-arm32"
        else
            DIDC_BINARY="didc-linux64"
        fi
    else
        echo "❌ Unsupported OS: $OS"
        exit 1
    fi

    DIDC_URL="https://github.com/dfinity/candid/releases/download/${DIDC_VERSION}/didc-${OS}-${ARCH}"

    echo "Downloading from: $DIDC_URL"
    curl -fsSL "$DIDC_URL" -o /tmp/didc
    chmod +x /tmp/didc

    # Try to install to /usr/local/bin, fallback to ~/.local/bin
    if sudo mv /tmp/didc /usr/local/bin/didc 2>/dev/null; then
        echo "Installed to /usr/local/bin/didc"
    else
        mkdir -p ~/.local/bin
        mv /tmp/didc ~/.local/bin/didc
        export PATH="$HOME/.local/bin:$PATH"
        echo "Installed to ~/.local/bin/didc (add to PATH if needed)"
    fi
fi

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