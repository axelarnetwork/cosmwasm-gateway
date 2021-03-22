use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{from_slice, CanonicalAddr, ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::{
    singleton, singleton_read, PrefixedStorage, ReadonlyPrefixedStorage, ReadonlySingleton,
    Singleton,
};

pub static OWNER_KEY: &[u8] = b"owner";
pub static PREFIX_TOKEN_ADDRESSES: &[u8] = b"command_addresses";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SomeSate {}

pub fn owner<S: Storage>(storage: &mut S) -> Singleton<S, CanonicalAddr> {
    singleton(storage, OWNER_KEY)
}

pub fn owner_read<S: Storage>(storage: &S) -> ReadonlySingleton<S, CanonicalAddr> {
    singleton_read(storage, OWNER_KEY)
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
