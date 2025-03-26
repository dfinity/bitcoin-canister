#!/usr/bin/env python3

import re
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
        r'prev_block_hash\s*=\s*blob\s*"([^"]+)";\s*}', re.DOTALL)

    blocks = []
    for match in block_pattern.finditer(text):
        height = int(match.group(1).replace("_", ""))
        block_hash = parse_blob(match.group(2))
        difficulty = int(match.group(3).replace("_", ""))
        children_raw = match.group(4)
        children_blobs = re.findall(r'blob\s*"([^"]+)"', children_raw)
        children = [parse_blob(b) for b in children_blobs]
        prev_block_hash = parse_blob(match.group(5))

        blocks.append({
            "height": height,
            "block_hash": block_hash,
            "difficulty": difficulty,
            "children": children,
            "prev_block_hash": prev_block_hash,
        })
    return {"data": blocks}

def main():
    # Load input from file or string
    with open("./unstable_blocks/output.txt", "r") as f:
        candid_text = f.read()

    parsed = parse_vec_block_data(candid_text)

    with open("./unstable_blocks/output.json", "w") as f:
        json.dump(parsed, f, indent=2)

    print("Parsed data saved to output.json")

if __name__ == "__main__":
    main()

# dfx canister call --network testnet bitcoin_t get_unstable_blocks > ./unstable_blocks/output.txt
