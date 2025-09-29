#!/usr/bin/env python3
import re
import sys
import json

def parse_blob(blob_str):
    """Convert blob with escaped characters to hex string."""
    matches = re.findall(r'\\([0-9a-fA-F]{2})', blob_str)
    return ''.join(matches)

def parse_vec_block_data(text):
    block_pattern = re.compile(
        r'record\s*{\s*height\s*=\s*([\d_]+)\s*:\s*nat;\s*'
        r'block_hash\s*=\s*blob\s*"([^"]+)";\s*'
        r'difficulty\s*=\s*([\d_]+)\s*:\s*nat;\s*'
        r'children\s*=\s*vec\s*{([^}]*)};\s*'
        r'no_difficulty_counter\s*=\s*([\d_]+)\s*:\s*nat;\s*'
        r'prev_block_hash\s*=\s*blob\s*"([^"]+)";\s*}', re.DOTALL)

    blocks = []
    for match in block_pattern.finditer(text):
        height = int(match.group(1).replace("_", ""))
        block_hash = parse_blob(match.group(2))
        difficulty = int(match.group(3).replace("_", ""))
        children_raw = match.group(4)
        children_blobs = re.findall(r'blob\s*"([^"]+)"', children_raw)
        children = [parse_blob(b) for b in children_blobs]
        no_difficulty_counter = int(match.group(5).replace("_", ""))
        prev_block_hash = parse_blob(match.group(6))

        blocks.append({
            "height": height,
            "difficulty": difficulty,
            "no_difficulty_counter": no_difficulty_counter,
            "prev_block_hash": prev_block_hash,
            "block_hash": block_hash,
            "children": children,
        })
    return {"data": blocks}

def main():
    if len(sys.argv) != 3:
        print("Usage: unstable_blocks.py <input.txt> <output.json>")
        sys.exit(1)

    with open(sys.argv[1], "r") as f:
        candid_text = f.read()

    parsed = parse_vec_block_data(candid_text)

    with open(sys.argv[2], "w") as f:
        json.dump(parsed, f, indent=2)

    print(f"Saved parsed output to {sys.argv[2]}")

if __name__ == "__main__":
    main()

# dfx canister call --network testnet bitcoin_t get_unstable_blocks > ./unstable_blocks/output.txt
