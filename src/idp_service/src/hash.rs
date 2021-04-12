//! Provides helper functions to calculate the representation independent hash of structured data.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize)]
enum Value {
    Bytes(#[serde(with = "serde_bytes")] Vec<u8>),
    String(String),
    U64(u64),
    Array(Vec<Value>),
}

#[allow(dead_code)]
fn hash_of_map<S: ToString>(map: &BTreeMap<S, Value>) -> [u8; 32] {
    let mut hashes: Vec<Vec<u8>> = Vec::new();
    for (key, val) in map.iter() {
        hashes.push(hash_key_val(key.to_string(), val.clone()));
    }

    // Computes hash by first sorting by "field name" hash, which is the
    // same as sorting by concatenation of H(field name) · H(field value)
    // (although in practice it's actually more stable in the presence of
    // duplicated field names). Then concatenate all the hashes.
    hashes.sort();

    let mut hasher = Sha256::new();
    for hash in hashes {
        hasher.update(&hash);
    }

    hasher.finalize().into()
}

fn hash_key_val(key: String, val: Value) -> Vec<u8> {
    let mut key_hash = hash_string(key);
    let mut val_hash = hash_val(val);
    key_hash.append(&mut val_hash);
    key_hash
}

fn hash_string(value: String) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hasher.finalize().to_vec()
}

fn hash_bytes(value: Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(&value);
    hasher.finalize().to_vec()
}

fn hash_u64(value: u64) -> Vec<u8> {
    // We need at most ⌈ 64 / 7 ⌉ = 10 bytes to encode a 64 bit
    // integer in LEB128.
    let mut buf = [0u8; 10];
    let mut n = value;
    let mut i = 0;

    loop {
        let byte = (n & 0x7f) as u8;
        n >>= 7;

        if n == 0 {
            buf[i] = byte;
            break;
        } else {
            buf[i] = byte | 0x80;
            i += 1;
        }
    }

    hash_bytes(buf[..=i].to_vec())
}

// Arrays encoded as the concatenation of the hashes of the encodings of the
// array elements.
fn hash_array(elements: Vec<Value>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    elements
        .into_iter()
        // Hash the encoding of all the array elements.
        .for_each(|e| hasher.update(hash_val(e).as_slice()));
    hasher.finalize().to_vec() // hash the concatenation of the hashes.
}

fn hash_val(val: Value) -> Vec<u8> {
    match val {
        Value::String(string) => hash_string(string),
        Value::Bytes(bytes) => hash_bytes(bytes),
        Value::U64(integer) => hash_u64(integer),
        Value::Array(elements) => hash_array(elements),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn message_id_icf_key_val_reference_1() {
        assert_eq!(
            hash_key_val(
                "request_type".to_string(),
                Value::String("call".to_string())
            ),
            hex!(
                "
                769e6f87bdda39c859642b74ce9763cdd37cb1cd672733e8c54efaa33ab78af9
                7edb360f06acaef2cc80dba16cf563f199d347db4443da04da0c8173e3f9e4ed
                "
            )
            .to_vec()
        );
    }

    #[test]
    fn message_id_u64_id_reference() {
        assert_eq!(
            // LEB128: 0x00
            hash_u64(0),
            hex!("6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"),
        );

        assert_eq!(
            // LEB128: 0xd2 0x09
            hash_u64(1234),
            hex!("8b37fd3ebbe6396a89ed8563dd0cc55927ac90138950460c77cffeb55cf63810"),
        );

        assert_eq!(
            // LEB128 0xff 0xff 0xff 0xff 0xff 0xff 0xff 0xff 0xff 0x01
            hash_u64(0xffff_ffff_ffff_ffff),
            hex!("51672ea45f3539654bf9193f4ff763d90022eee7df5f5b76353d6f11a9eaccec"),
        )
    }

    #[test]
    fn message_id_string_reference_1() {
        assert_eq!(
            hash_string("request_type".to_string()),
            hex!("769e6f87bdda39c859642b74ce9763cdd37cb1cd672733e8c54efaa33ab78af9"),
        );
    }

    #[test]
    fn message_id_string_reference_2() {
        assert_eq!(
            hash_string("call".to_string()),
            hex!("7edb360f06acaef2cc80dba16cf563f199d347db4443da04da0c8173e3f9e4ed"),
        );
    }

    #[test]
    fn message_id_string_reference_3() {
        assert_eq!(
            hash_string("callee".to_string()),
            hex!("92ca4c0ced628df1e7b9f336416ead190bd0348615b6f71a64b21d1b68d4e7e2"),
        );
    }

    #[test]
    fn message_id_string_reference_4() {
        assert_eq!(
            hash_string("method_name".to_string()),
            hex!("293536232cf9231c86002f4ee293176a0179c002daa9fc24be9bb51acdd642b6"),
        );
    }

    #[test]
    fn message_id_string_reference_5() {
        assert_eq!(
            hash_string("hello".to_string()),
            hex!("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"),
        );
    }

    #[test]
    fn message_id_string_reference_6() {
        assert_eq!(
            hash_string("arg".to_string()),
            hex!("b25f03dedd69be07f356a06fe35c1b0ddc0de77dcd9066c4be0c6bbde14b23ff"),
        );
    }

    #[test]
    fn message_id_array_reference_1() {
        assert_eq!(
            hash_array(vec![Value::String("a".to_string())]),
            // hash(hash("a"))
            hex!("bf5d3affb73efd2ec6c36ad3112dd933efed63c4e1cbffcfa88e2759c144f2d8"),
        );
    }

    #[test]
    fn message_id_array_reference_2() {
        assert_eq!(
            hash_array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
            ]),
            // hash(concat(hash("a"), hash("b"))
            hex!("e5a01fee14e0ed5c48714f22180f25ad8365b53f9779f79dc4a3d7e93963f94a"),
        );
    }

    #[test]
    fn message_id_array_reference_3() {
        assert_eq!(
            hash_array(vec![
                Value::Bytes(vec![97]), // "a" as a byte string.
                Value::String("b".to_string()),
            ]),
            // hash(concat(hash("a"), hash("b"))
            hex!("e5a01fee14e0ed5c48714f22180f25ad8365b53f9779f79dc4a3d7e93963f94a"),
        );
    }

    #[test]
    fn message_id_array_reference_4() {
        assert_eq!(
            hash_array(vec![Value::Array(vec![Value::String("a".to_string())])]),
            // hash(hash(hash("a"))
            hex!("eb48bdfa15fc43dbea3aabb1ee847b6e69232c0f0d9705935e50d60cce77877f"),
        );
    }

    #[test]
    fn message_id_array_reference_5() {
        assert_eq!(
            hash_array(vec![Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string())
            ])]),
            // hash(hash(concat(hash("a"), hash("b")))
            hex!("029fd80ca2dd66e7c527428fc148e812a9d99a5e41483f28892ef9013eee4a19"),
        );
    }

    #[test]
    fn message_id_array_reference_6() {
        assert_eq!(
            hash_array(vec![
                Value::Array(vec![
                    Value::String("a".to_string()),
                    Value::String("b".to_string())
                ]),
                Value::Bytes(vec![97]), // "a" in bytes
            ]),
            // hash(concat(hash(concat(hash("a"), hash("b")), hash(100))
            hex!("aec3805593d9ec6df50da070597f73507050ce098b5518d0456876701ada7bb7"),
        );
    }

    #[test]
    fn message_id_bytes_reference() {
        assert_eq!(
            // D    I    D    L    \0   \253 *"
            // 68   73   68   76   0    253  42
            hash_bytes(vec![68, 73, 68, 76, 0, 253, 42]),
            hex!("6c0b2ae49718f6995c02ac5700c9c789d7b7862a0d53e6d40a73f1fcd2f70189")
        );
    }
}