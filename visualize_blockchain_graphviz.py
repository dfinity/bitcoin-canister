#!/usr/bin/env python3
import json
import sys
from graphviz import Digraph

def short_hash(hash_hex):
    return hash_hex[:6]

def load_blocks(file_path):
    with open(file_path, "r") as f:
        data = json.load(f)
    return data["data"]

def difficulty_to_color(difficulty, min_difficulty, max_difficulty):
    if max_difficulty == min_difficulty:
        ratio = 0.0
    else:
        ratio = (difficulty - min_difficulty) / (max_difficulty - min_difficulty)

    # Interpolate between light blue (232, 240, 254) and dark blue (0, 60, 143)
    r = int(232 + ratio * (0 - 232))
    g = int(240 + ratio * (60 - 240))
    b = int(254 + ratio * (143 - 254))
    return f"#{r:02x}{g:02x}{b:02x}"

def generate_graph(blocks, output_file="blockchain"):
    dot = Digraph(comment="Blockchain", format="png")

    dot.attr(
        rankdir="TB",
        dpi="300",
        nodesep="0.4",
        ranksep="0.5",
        bgcolor="white"
    )

    # Add top and bottom dummy nodes to pad the layout area
    dot.node("padding_top", "", shape="box", width="4.0", height="0", style="invis")
    dot.node("padding_bottom", "", shape="box", width="4.0", height="0", style="invis")

    difficulties = [b["difficulty"] for b in blocks]
    min_diff = min(difficulties)
    max_diff = max(difficulties)

    hash_to_block = {b["block_hash"]: b for b in blocks}

    first_hash = blocks[0]["block_hash"]
    last_hash = blocks[-1]["block_hash"]

    for block in blocks:
        short = short_hash(block["block_hash"])
        label = f"#{short}\\nH:{block['height']}\\nD:{block['difficulty']}\\nC:{block['no_difficulty_counter']}"
        fillcolor = difficulty_to_color(block["difficulty"], min_diff, max_diff)

        dot.node(
            block["block_hash"],
            label=label,
            shape="box",
            style="filled",
            fillcolor=fillcolor,
            fontsize="10",
            width="2.0",
            height="0.6",
            fontname="Helvetica"
        )

    for block in blocks:
        for child in block["children"]:
            if child in hash_to_block:
                dot.edge(block["block_hash"], child)

    # Add invisible edges to force spacing
    dot.edge("padding_top", first_hash, style="invis")
    dot.edge(last_hash, "padding_bottom", style="invis")

    dot.render(output_file, cleanup=True)
    print(f"Graph written to {output_file}.png")

def main():
    if len(sys.argv) != 3:
        print("Usage: visualize_blockchain_graphviz.py <input.json> <output.png>")
        sys.exit(1)

    blocks = load_blocks(sys.argv[1])
    output_base = sys.argv[2].rsplit(".", 1)[0]
    generate_graph(blocks, output_base)

if __name__ == "__main__":
    main()

# sudo pip3 install graphviz
