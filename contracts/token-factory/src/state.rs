use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, HumanAddr, ReadonlyStorage, StdError, StdResult, Storage, from_slice};
use cosmwasm_storage::{
    singleton, singleton_read, PrefixedStorage, ReadonlyPrefixedStorage
};

pub static KEY_CONFIG: &[u8] = b"owner";
pub static PREFIX_TOKEN_ADDRESSES: &[u8] = b"token_addresses";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub token_code_id: u64,
}

pub fn store_config<S: Storage>(storage: &mut S, data: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(data)
}

pub fn read_config<S: Storage>(storage: &S) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

pub fn store_token_address<S: Storage>(
    storage: &mut S,
    symbol: &String,
    address: &CanonicalAddr,
) -> StdResult<()> {
    PrefixedStorage::new(PREFIX_TOKEN_ADDRESSES, storage)
        .set(symbol.as_bytes(), address.as_slice());

    Ok(())
}

pub fn read_token_address<S: Storage>(
    storage: &S,
    symbol: &String,
) -> StdResult<CanonicalAddr> {
    let res = ReadonlyPrefixedStorage::new(PREFIX_TOKEN_ADDRESSES, storage).get(symbol.as_bytes());
    match res {
        Some(data) => Ok(CanonicalAddr::from(data)),
        None => Err(StdError::generic_err(
            "no registered token address",
        )),
    }
}
