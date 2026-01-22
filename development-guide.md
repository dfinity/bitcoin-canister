# Development Guide

## Release Overview

This repository contains multiple packages with different release strategies:

| Package             | Versioning                                        | Published on crates.io? |
|---------------------|---------------------------------------------------|-------------------------|
| `ic-btc-canister`   | Date-based (`ic-btc-canister/release/YYYY-MM-DD`) | No                      |
| `watchdog`          | Date-based (`watchdog/release/YYYY-MM-DD`)        | No                      |
| `ic-btc-interface`  | Semver (`X.Y.Z`)                                  | Yes                     |
| `ic-btc-validation` | Semver (`X.Y.Z`)                                  | Yes                     |

### Canister IDs

**Bitcoin canister:**

| Network         | Production                    | Staging                       |
|-----------------|-------------------------------|-------------------------------|
| Bitcoin Mainnet | `ghsi2-tqaaa-aaaan-aaaca-cai` | `axowo-ciaaa-aaaad-acs7q-cai` |
| Bitcoin Testnet | `g4xu7-jiaaa-aaaan-aaaaq-cai` | -                             |

**Watchdog canister:**

| Network          | Production                    | Staging                       |
|------------------|-------------------------------|-------------------------------|
| Bitcoin Mainnet  | `gatoo-6iaaa-aaaan-aaacq-cai` | `ljyeq-zaaaa-aaaad-actaa-cai` |
| Bitcoin Testnet  | `gjqfs-iaaaa-aaaan-aaada-cai` | -                             |
| Dogecoin Mainnet | `he6b4-hiaaa-aaaan-aaaeq-cai` | `kwqxh-2yaaa-aaaad-acteq-cai` |
| Dogecoin Testnet | `hn5ka-raaaa-aaaan-aaafa-cai` | -                             |

The Bitcoin and watchdog canisters are deployed in production by submitting proposals to the Internet
Computer's [Network Nervous System](https://internetcomputer.org/nns).

## Releasing Canisters (ic-btc-canister / watchdog)

### Step 1: Create a Release PR

1. Go to Actions → Create Release PR
2. Click **Run workflow**
3. Select the canister (`ic-btc-canister` or `watchdog`)
4. Click **Run workflow**

This creates a draft PR that updates the canister's `CHANGELOG.md` using [git-cliff](https://git-cliff.org/).

5. Review and merge the PR

### Step 2: Create GitHub Release

1. Go to Actions → Create GitHub Releases
2. Click **Run workflow**
3. Select the canister (`ic-btc-canister` or `watchdog`)
4. Click **Run workflow**

This creates a **draft** GitHub release with:

- WASM artifact (downloaded from latest CI build on `master`)
- Candid file
- Changelog (scoped to the package's directory)
- SHA-256 checksum
- Placeholder for NNS proposal links

5. Review the draft release

### Step 3: Deploy via NNS Proposal

After the release is published:

1. Submit an NNS proposal to upgrade/re-install the canister
2. Update the release notes with the proposal link
3. Mark the release as "Latest" once deployed

## Releasing Library Crates (ic-btc-interface / ic-btc-validation)

### Step 1: Create a Release PR

1. Go to Actions → Create Release PR
2. Click **Run workflow**
3. Select `library-crates`
4. Click **Run workflow**

This uses [release-plz](https://release-plz.ieni.dev/) to create a PR that:

- Bumps versions in `Cargo.toml` based on conventional commits (patch, minor, or major)
- Updates `CHANGELOG.md` for both crates

5. Review and merge the PR

### Step 2: Publish to crates.io

1. Go to Actions → Publish Crates to crates.io
2. Click **Run workflow**

This publishes both `ic-btc-interface` and `ic-btc-validation` to crates.io and creates git tags.

## Manual WASM Build (for verification)

To manually build and verify WASM checksums:

```shell
# Clone and checkout the release commit
git clone https://github.com/dfinity/bitcoin-canister
cd bitcoin-canister
git checkout <commit-sha>

# Build reproducibly with Docker
./scripts/docker-build

# Verify checksums match the release
sha256sum *.wasm.gz
```

**Note**: Reproducible builds require Docker. There is no reproducibility guarantee on Mac M1s; preferably use Ubuntu or
Intel Macs.
