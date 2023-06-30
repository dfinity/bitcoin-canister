use ic_btc_interface::Txid;
use proptest::array::uniform32;
use proptest::collection::vec as pvec;
use proptest::prelude::*;
use serde_bytes::ByteBuf;
use std::str::FromStr;

proptest! {
    #[test]
    fn txid_display_roundtrip(bytes in uniform32(any::<u8>())) {
        let txid = Txid::from(bytes);
        prop_assert_eq!(txid, Txid::from_str(&txid.to_string()).unwrap());
    }

    #[test]
    fn serde_cbor_decode_byte_array(bytes in uniform32(any::<u8>())) {
        let mut encoded = vec![];
        ciborium::into_writer(&bytes, &mut encoded).unwrap();
        let txid: Txid = ciborium::from_reader(&encoded[..]).unwrap();
        prop_assert_eq!(txid, Txid::from(bytes));
    }

    #[test]
    fn serde_cbor_decode_bytebuf(bytes in pvec(any::<u8>(), 32)) {
        let mut encoded = vec![];
        let original_txid = Txid::try_from(&bytes[..]).unwrap();
        ciborium::into_writer(&ByteBuf::from(bytes), &mut encoded).unwrap();
        let txid: Txid = ciborium::from_reader(&encoded[..]).unwrap();
        prop_assert_eq!(txid, original_txid);
    }
}
