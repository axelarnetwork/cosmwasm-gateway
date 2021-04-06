use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, CanonicalAddr, HumanAddr, ReadonlyStorage, StdError, StdResult, Storage, from_binary, to_binary};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage, ReadonlySingleton, Singleton, singleton, singleton_read};
use k256::{ecdsa::VerifyingKey, CompressedPoint};
use cosmwasm_crypto::{ECDSA_PUBKEY_MAX_LEN};

/// Length of a serialized compressed public key
const ECDSA_COMPRESSED_PUBKEY_LEN: usize = 33;

pub static KEY_CONFIG: &[u8] = b"config";
pub static PREFIX_META_TX_NONCE: &[u8] = b"meta_nonces";
pub static PREFIX_CONTRACT_ADDRESSES: &[u8] = b"contract_addresses";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    // contract owner address
    pub owner: CanonicalAddr,
    // k256::CompressedPoint Compressed SEC1-encoded secp256k1 (K-256) point.
    pub public_key: Vec<u8>,

    pub nonce: u64, // prevent replay of meta-transactions

    // address of secp256k1 signature verification contract
    pub crypto_contract_addr: CanonicalAddr,

    // freeze gateway
    pub mutable: bool,
}

/// Convert a SEC1-encoded compressed secp256k1 point (public key bytes) to a base64 string
pub fn base64_str_from_sec1_bytes(pub_key: &CompressedPoint) -> String {
    let vec = &pub_key.to_vec();
    let bin = to_binary(pub_key.as_ref()).unwrap();
    bin.to_base64()
}

/// Convert a base64 string representing a SEC1-encoded compressed secp256k1 point to a verifying
/// key.
pub fn verifying_key_from_base64_str(pk_str: &str) -> StdResult<VerifyingKey> {
    let bin = Binary::from_base64(pk_str)?;
    let key_vec: Vec<u8> = from_binary(&bin)?;
    match VerifyingKey::from_sec1_bytes(key_vec.as_slice()) {
        Ok(vk) => Ok(vk),
        Err(err) => return Err(StdError::generic_err("failed to deserialize public key")),
    }
}

pub fn store_config<S: Storage>(storage: &mut S, data: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(data)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_contract_address<S: Storage>(
    storage: &mut S,
    name: &String,
    address: &CanonicalAddr,
) -> StdResult<()> {
    PrefixedStorage::new(PREFIX_CONTRACT_ADDRESSES, storage)
        .set(name.as_bytes(), address.as_slice());

    Ok(())
}

pub fn read_contract_address<S: Storage>(
    storage: &S,
    name: &String,
) -> StdResult<CanonicalAddr> {
    let res = ReadonlyPrefixedStorage::new(PREFIX_CONTRACT_ADDRESSES, storage).get(name.as_bytes());
    match res {
        Some(data) => Ok(CanonicalAddr::from(data)),
        None => Err(StdError::generic_err(
            "no registered contract address",
        )),
    }
}

impl Config {
    pub fn update_owner(&mut self, owner: CanonicalAddr, public_key: &String) -> StdResult<()> {
        // sanitize pub_key
        let verifying_key = verifying_key_from_base64_str(public_key)?;

        // cfg.update_owner(deps.api.canonical_address(&msg.owner)?, pub_key.to_bytes().to_vec());
        self.public_key = verifying_key.to_bytes().to_vec();
        self.owner = owner;
        
        // @nb messages could be replayed if owner was set to a previous owner
        // We would need to store map of owners to nonces to prevent this.
        self.nonce = 0; // reset nonce
        Ok(())
    }

    pub fn increment_nonce(&mut self) -> &u64 {
        self.nonce += 1;
        return &self.nonce;
    }
}

