# Example Canister

This is a canister running on the Internet Computer (IC) platform that demonstrates how to make an HTTPS outcall to a simple dummyjson.com API to fetch a quote and apply a transform function to the HTTP response. The transform function extracts only the author name, and the result is presented as a string.


## Running the project locally

To run this project locally, you can use the following commands:

```bash
# Stop any currently running replica
dfx stop

# Start the replica, running in the background
dfx start --background --clean

# Deploy your canisters to the replica and generate your Candid interface
dfx deploy

# After some time, the UI interface will be available via a link similar to this
  Backend canister via Candid interface:
    canister_backend: http://127.0.0.1:4943/?canisterId=ryjl3-tyaaa-aaaaa-aaaba-cai&id=rrkah-fqaaa-aaaaa-aaaaq-cai
```
