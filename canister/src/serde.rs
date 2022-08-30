use bitcoin::{
    consensus::{Decodable, Encodable},
    Block,
};
use serde::{Deserializer, Serializer};

/// A method for serde to serialize a block.
/// Serialization relies on converting the block into a blob using the
/// Bitcoin standard format.
pub fn serialize_block<S>(block: &Block, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut bytes = vec![];
    Block::consensus_encode(block, &mut bytes).unwrap();
    serde_bytes::serialize(&bytes, s)
}

/// A method for serde to deserialize a block.
/// The blob is assumed to be in Bitcoin standard format.
pub fn deserialize_block<'de, D>(d: D) -> Result<Block, D::Error>
where
    D: Deserializer<'de>,
{
    let bytes: Vec<u8> = serde_bytes::deserialize(d).unwrap();
    Ok(Block::consensus_decode(bytes.as_slice()).unwrap())
}
