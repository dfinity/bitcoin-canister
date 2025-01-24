#!/usr/bin/python3

import re
import csv
from datetime import datetime

# Input and output file names
log_file = "canister.log"
csv_file = "canister-log.csv"

# Fields to extract from logs
humeric_fields = [
    "main_chain_height",
    "stable_height",
    "utxos_length",
    "address_utxos_length",
    "anchor_difficulty",
    "normalized_stability_threshold",
    "unstable_blocks_num_tips",
    "unstable_blocks_total",
    "unstable_blocks_depth",
    "unstable_blocks_difficulty_based_depth",
    "stable_memory_size_in_bytes",
    "heap_size_in_bytes",
    "num_get_successors_rejects",
    "num_block_deserialize_errors",
    "num_insert_block_errors",
    "send_transaction_count",
    # "cycles_burnt",
    # "cycles_balance",
]

fields = humeric_fields + [
    "is_synced",
    "complete_response_blocks", 
    "complete_response_size",
    "sending_request",
    "ingested_stable_blocks",
    "id",
]

def init_data():
    return {field: None for field in fields}

# Regex patterns
log_pattern = re.compile(r"\[(\d+)\. (\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{6})\d+Z\]: (.*)")

# Parse logs
relative_start_time = None
current_data = init_data()
parsed_data = []
ingested_blocks_count = None

with open(log_file, "r") as file:
    for line in file:
        match = log_pattern.match(line)
        if match:
            current_id = int(match.group(1))
            timestamp_raw, message = match.group(2), match.group(3)
            timestamp = datetime.strptime(timestamp_raw, "%Y-%m-%dT%H:%M:%S.%f")

            # Set the start time
            if relative_start_time is None:
                relative_start_time = timestamp

            if "Starting heartbeat..." in message:
                # Append the current data to parsed_data if all fields are populated
                if any(value is not None for key, value in current_data.items() if key != "time"):
                    parsed_data.append(current_data.copy())
                    current_data = init_data()

            # Calculate relative time
            relative_time = timestamp - relative_start_time
            relative_time_str = str(relative_time).split(".")[0] + f".{relative_time.microseconds // 1000:03d}"
            if current_data.get("time") != relative_time_str:
                current_data = init_data()
                current_data["id"] = current_id
                current_data["time"] = relative_time_str

            # Extract fields from the message
            for field in humeric_fields:
                field_match = re.search(rf"{field}: (\d+)", message)
                if field_match:
                    current_data[field] = int(field_match.group(1))

            field = "is_synced"
            field_match = re.search(rf"{field}: (true|false)", message)
            if field_match:
                current_data[field] = 1 if field_match.group(1) == "true" else 0

            field_match = re.search(rf"Sending request: ", message)
            if field_match:
                current_data["sending_request"] = int(1)

            field_match = re.search(rf"Received complete response with (\d+) blocks of size (\d+) bytes", message)
            if field_match:
                current_data["complete_response_blocks"] = int(field_match.group(1))
                current_data["complete_response_size"] = int(field_match.group(2))

            field_match = re.search(rf"Ingesting new stable block", message)
            if field_match:
                if ingested_blocks_count is None:
                    ingested_blocks_count = 0
                ingested_blocks_count += 1
            field_match = re.search(rf"Done ingesting stable blocks", message)
            if field_match:
                current_data["ingested_stable_blocks"] = ingested_blocks_count
                ingested_blocks_count = None


# Write to CSV
with open(csv_file, "w", newline="") as file:
    writer = csv.writer(file, delimiter="\t")
    writer.writerow(["time"] + fields)
    for data in parsed_data:
        writer.writerow([data.get("time", "")] + [data.get(field, "") for field in fields])

print(f"CSV file '{csv_file}' has been created.")
