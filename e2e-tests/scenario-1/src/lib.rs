pub const MINER_ADDRESS: &str = "mwSSBD3NCriNXNMgd6dr2N2rxX9M9zXqrp";
pub const ADDRESS_1: &str = "bcrt1qg4cvn305es3k8j69x06t9hf4v5yx4mxdaeazl8";
pub const ADDRESS_2: &str = "bcrt1qxp8ercrmfxlu0s543najcj6fe6267j97tv7rgf";
pub const ADDRESS_3: &str = "bcrt1qp045tvzkxx0292645rxem9eryc7jpwsk3dy60h";
pub const ADDRESS_4: &str = "bcrt1qjft8fhexv4znxu22hed7gxtpy2wazjn0x079mn";
pub const ADDRESS_5: &str = "bcrt1qenhfslne5vdqld0djs0h0tfw225tkkzzc60exh";

use candid::CandidType;
use serde::Deserialize;

/// Outcome of a `proxy_call`: how many cycles the downstream callee charged
/// (cycles attached minus cycles refunded) and the reject message, if the
/// call was rejected. Used by e2e tests to observe the bitcoin canister's
/// cycle-charging behaviour, which ingress messages cannot exercise since
/// they carry no cycles.
#[derive(CandidType, Deserialize, Debug)]
pub struct ProxyCallResult {
    pub charged_cycles: u128,
    pub reject_message: Option<String>,
}
