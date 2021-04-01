use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Binary, CanonicalAddr, HumanAddr, StdError, StdResult, Storage};
use cosmwasm_storage::{ReadonlyPrefixedStorage, ReadonlySingleton, Singleton, singleton, singleton_read};
use k256::{CompressedPoint};
use cosmwasm_crypto::{ECDSA_PUBKEY_MAX_LEN};

/// Length of a serialized compressed public key
const ECDSA_COMPRESSED_PUBKEY_LEN: usize = 33;

pub static KEY_CONFIG: &[u8] = b"config";
pub static PREFIX_META_TX_NONCE: &[u8] = b"meta_nonces";

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

pub fn store_config<S: Storage>(storage: &mut S, data: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(data)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

impl Config {
    pub fn update_owner(&mut self, owner: CanonicalAddr, public_key: Vec<u8>) -> StdResult<()> {
        if !(public_key.len() == ECDSA_PUBKEY_MAX_LEN || public_key.len() == ECDSA_COMPRESSED_PUBKEY_LEN) {
            return Err(StdError::generic_err("invalid ECDSA public key"));
        }
        self.public_key = public_key;
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

