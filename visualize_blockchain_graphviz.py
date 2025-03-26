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

def generate_graph(blocks, output_file="blockchain"):
    dot = Digraph(comment="Blockchain", format="png")
    dot.attr(rankdir="TB")  # Top to bottom layout
    dot.attr(dpi="150")     # Higher resolution

    node_style = {
        "shape": "box",
        "style": "filled",
        "fillcolor": "#e8f0fe",
        "fontsize": "10",
        "width": "1.5",
        "height": "0.6",
        "fontname": "Helvetica"
    }

    hash_to_block = {b["block_hash"]: b for b in blocks}

    # Add nodes with labels
    for block in blocks:
        short = short_hash(block["block_hash"])
        label = f"#{short}\\nH:{block['height']}\\nD:{block['difficulty']}"
        dot.node(block["block_hash"], label, **node_style)

    # Add edges (parent -> child)
    for block in blocks:
        for child in block["children"]:
            if child in hash_to_block:
                dot.edge(block["block_hash"], child)

    # Save to file
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
