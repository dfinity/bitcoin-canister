rm -rf .dfx
dfx start --background
dfx deploy --no-wallet management_mock

# Deploy the bitcoin canister
dfx deploy --no-wallet bitcoin --argument "(record { stability_threshold = 2; network = variant { regtest }; management_canister = principal \"$(dfx canister id management_mock)\" })"

sleep 5

BALANCE=$(dfx canister call bitcoin get_balance '(record { address = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8" })')

dfx stop

if [[ $BALANCE = "(5_000_000_000 : nat64)" ]]; then
  echo "SUCCESS"
  exit 0
else
  echo "FAIL"
  exit 1
fi
