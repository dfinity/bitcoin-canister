use ic_btc_interface::Txid;
use proptest::array::uniform32;
use proptest::prelude::*;
use std::str::FromStr;

proptest! {
    #[test]
    fn txid_display_roundtrip(bytes in uniform32(any::<u8>())) {
        let txid = Txid::from(bytes);
        prop_assert_eq!(txid, Txid::from_str(&txid.to_string()).unwrap());
    }

    #[test]
    fn serde_cbor_decode(bytes in uniform32(any::<u8>())) {
        let mut encoded = vec![];
        ciborium::into_writer(&bytes, &mut encoded).unwrap();
        let txid: Txid = ciborium::from_reader(&encoded[..]).unwrap();
        prop_assert_eq!(txid, Txid::from(bytes));
    }
}
