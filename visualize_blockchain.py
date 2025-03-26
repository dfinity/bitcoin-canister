#!/usr/bin/env python3

import json
import matplotlib.pyplot as plt
import networkx as nx

def short_hash(hash_hex):
    return hash_hex[:6]

def load_blocks(file_path):
    with open(file_path, "r") as f:
        data = json.load(f)
    return data["data"]

def build_graph(blocks):
    G = nx.DiGraph()

    hash_to_block = {b["block_hash"]: b for b in blocks}

    for block in blocks:
        label = f'H:{block["height"]} D:{block["difficulty"]} #{short_hash(block["block_hash"])}'
        G.add_node(block["block_hash"], label=label)

    for block in blocks:
        for child_hash in block["children"]:
            if child_hash in hash_to_block:
                G.add_edge(block["block_hash"], child_hash)

    return G

def draw_graph(G):
    pos = nx.spring_layout(G, seed=42)  # You can try different layouts like graphviz_layout if you have pygraphviz
    labels = nx.get_node_attributes(G, 'label')

    plt.figure(figsize=(14, 10))
    nx.draw_networkx_nodes(G, pos, node_size=800, node_color='lightblue')
    nx.draw_networkx_edges(G, pos, arrows=True, arrowstyle='->')
    nx.draw_networkx_labels(G, pos, labels, font_size=8)

    plt.title("Blockchain Visualization")
    plt.axis("off")
    plt.tight_layout()
    plt.show()

def main():
    blocks = load_blocks("./unstable_blocks/output.json")
    G = build_graph(blocks)
    draw_graph(G)

if __name__ == "__main__":
    main()
