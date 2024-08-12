echo -n pin:; read -s dfx_hsm_pin; export dfx_hsm_pin; ic-admin \
    --use-hsm \
    --key-id 01 \
    --slot 0 \
    --pin "$dfx_hsm_pin" \
    --nns-url "https://ic0.app" \
    propose-to-change-nns-canister \
    --proposer 50 \
    --args args.bin \
    --mode upgrade \
    --canister-id ghsi2-tqaaa-aaaan-aaaca-cai \
    --wasm-module-path ./ic-btc-canister.wasm.gz \
    --wasm-module-sha256 f32f25f2e04d2c45600854aabf13e1d75f38197d7a7660fd17023cbcd0fcf9a6 \
    --summary-file ./proposal-summary
