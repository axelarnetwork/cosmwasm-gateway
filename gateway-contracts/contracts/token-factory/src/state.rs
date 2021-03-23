use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{from_slice, CanonicalAddr, ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::{
    singleton, singleton_read, PrefixedStorage, ReadonlyPrefixedStorage
};

pub static KEY_CONFIG: &[u8] = b"owner";
pub static PREFIX_TOKEN_ADDRESSES: &[u8] = b"command_addresses";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub token_code_id: u64,
}

pub fn config_store<S: Storage>(storage: &mut S, data: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(data)
}

pub fn config_read<S: Storage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn token_address_store<S: Storage>(
    storage: &mut S,
    token_symbol: &String,
    address: &CanonicalAddr,
) -> StdResult<()> {
    PrefixedStorage::new(PREFIX_TOKEN_ADDRESSES, storage)
        .set(token_symbol.as_bytes(), address.as_slice());

    Ok(())
}

pub fn token_address_read<S: Storage>(
    storage: &S,
    token_symbol: &String,
) -> StdResult<CanonicalAddr> {
    let res =
        ReadonlyPrefixedStorage::new(PREFIX_TOKEN_ADDRESSES, storage).get(token_symbol.as_bytes());
    match res {
        Some(data) => from_slice(&data),
        None => Err(StdError::generic_err(
            "there is no registered token address",
        )),
    }
}
