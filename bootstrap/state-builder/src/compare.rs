use bitcoin::OutPoint as BitcoinOutPoint;
use ic_btc_canister::types::{Address, AddressUtxo, OutPoint, Storable as _, TxOut};
use ic_btc_types::Height;
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager},
    DefaultMemoryImpl, FileMemory, StableBTreeMap,
};
use std::fs::File;

fn main() {
    let balances_script_mem = FileMemory::new(File::open("./testnet_balances_100k").unwrap());

    let mut balances: StableBTreeMap<_, Address, u64> =
        StableBTreeMap::init(balances_script_mem, 90, 8);

    println!("# balances {}", balances.len());

    let canister_mem = FileMemory::new(File::open("./testnet_100k_reference.bin").unwrap());

    let memory_manager = MemoryManager::init(canister_mem);

    let address_utxos_reference = memory_manager.get(MemoryId::new(1));
    let balance_mem_reference = memory_manager.get(MemoryId::new(4));

    let mut balances_reference: StableBTreeMap<_, Address, u64> =
        StableBTreeMap::init(balance_mem_reference, 90, 8);

    println!("# balances in ref {}", balances_reference.len());

    assert_eq!(balances_reference.len(), balances.len());

    for ((k1, v1), (k2, v2)) in std::iter::zip(balances.iter(), balances_reference.iter()) {
        assert_eq!(k1, k2);
        assert_eq!(v1, v2);
    }

    let address_utxos_reference: StableBTreeMap<_, AddressUtxo, ()> = StableBTreeMap::init(
        address_utxos_reference,
        90 + 36, // max outpoint size.
        0,       // No values are stored in the map.
    );

    let address_utxos = FileMemory::new(File::open("./address_utxos").unwrap());

    let mut address_utxos: StableBTreeMap<_, AddressUtxo, ()> =
        StableBTreeMap::init(address_utxos, 90, 8);

    println!("# address utxos: {}", address_utxos.len());
    println!(
        "# address utxos reference: {}",
        address_utxos_reference.len()
    );

    for ((k1, v1), (k2, v2)) in std::iter::zip(address_utxos.iter(), address_utxos_reference.iter())
    {
        assert_eq!(k1, k2);
        assert_eq!(v1, v2);
    }

    let small_utxos = FileMemory::new(File::open("./small_utxos").unwrap());

    let mut small_utxos: StableBTreeMap<_, Vec<u8>, Vec<u8>> =
        StableBTreeMap::init(small_utxos, 0, 0);

    let small_utxos_reference = memory_manager.get(MemoryId::new(2));
    let mut small_utxos_reference: StableBTreeMap<_, Vec<u8>, Vec<u8>> =
        StableBTreeMap::init(small_utxos_reference, 0, 0);

    println!("# small utxos: {}", small_utxos.len());
    println!("# small utxos referenced: {}", small_utxos_reference.len());

    for (i, ((k1, v1), (k2, v2))) in
        std::iter::zip(small_utxos.iter(), small_utxos_reference.iter()).enumerate()
    {
        assert_eq!(k1, k2);
        assert_eq!(v1, v2);
    }

    let medium_utxos = FileMemory::new(File::open("./medium_utxos").unwrap());

    let mut medium_utxos: StableBTreeMap<_, Vec<u8>, Vec<u8>> =
        StableBTreeMap::init(medium_utxos, 0, 0);

    let medium_utxos_reference = memory_manager.get(MemoryId::new(3));
    let mut medium_utxos_reference: StableBTreeMap<_, Vec<u8>, Vec<u8>> =
        StableBTreeMap::init(medium_utxos_reference, 0, 0);

    println!("# medium utxos: {}", medium_utxos.len());
    println!(
        "# medium utxos referenced: {}",
        medium_utxos_reference.len()
    );

    for (i, ((k1, v1), (k2, v2))) in
        std::iter::zip(medium_utxos.iter(), medium_utxos_reference.iter()).enumerate()
    {
        let k1 = OutPoint::from_bytes(k1);
        let k2 = OutPoint::from_bytes(k2);
        if k1 != k2 {
            println!("{:?}, {:?}", k1, k2);

            let v1 = <(TxOut, Height)>::from_bytes(v1.clone());
            println!(
                "script: {:?}",
                bitcoin::Script::from(v1.clone().0.script_pubkey)
            );
            println!(
                "is provably unspendable? {:?}",
                bitcoin::Script::from(v1.clone().0.script_pubkey).is_provably_unspendable()
            );

            /*let k1: BitcoinOutPoint = BitcoinOutPoint {
                txid: bitcoin::Txid::from_hash(
                    bitcoin::hashes::Hash::from_slice(k1.txid.as_bytes())
                        .expect("txid must be valid"),
                ),
                vout: k1.vout,
            };
            let k2: BitcoinOutPoint = BitcoinOutPoint {
                txid: bitcoin::Txid::from_hash(
                    bitcoin::hashes::Hash::from_slice(k2.txid.as_bytes())
                        .expect("txid must be valid"),
                ),
                vout: k2.vout,
            };

            println!(
                "is provably unspendable: {:?}",
                k1.txid.script_pubkey.is_provably_unspendable()
            );*/
        }
        assert_eq!(k1, k2);
        if v1 != v2 {
            println!("{:?}, {:?}", k1, k2);
            let v1 = <(TxOut, Height)>::from_bytes(v1.clone());
            let v2 = <(TxOut, Height)>::from_bytes(v2.clone());
            println!("{:?}, {:?}", v1, v2);
        }

        assert_eq!(v1, v2);
    }
}
