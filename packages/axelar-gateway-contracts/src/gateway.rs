use std::fmt;

use cosmwasm_std::{CosmosMsg, Empty, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
// InitMsg accepts the owner's address and public key as parameters instead of
// using the message sender as the contract owner to verify gateway deployment, 
// Axelar must query the owner address and pubkey
pub struct InitMsg {
    pub owner: HumanAddr,
    pub public_key: String,
    pub crypto_contract_addr: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. 
    /// Any instantiated contracts that need to be registered must be
    /// listed in the [[register]] field.
    Execute { msgs: Vec<CosmosMsg<T>>, register: Vec<String>},

    /// Same as Execute except authorization is performed by verifying the provided
    /// signature was created by the contract owner. ExecuteSigned messages can be
    /// sent by anyone.
    ExecuteSigned { sig: Vec<u8>, msgs: Vec<CosmosMsg<T>>, register: Vec<String> },

    /// Receive hook from instantiated contract to register its address
    Register { name: String },

    /// Freeze will make the contract immutable. Must be called by the owner.
    Freeze {},

    /// UpdateOwner will change the admin set of the contract, must be called by the existing
    /// owner, and only works if the contract is mutable.
    UpdateOwner { owner: HumanAddr, public_key: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Retrieve the contract's configuration
    Config {},

    /// Retrieve the address of a registered contract
    ContractAddress { name: String },

    /// Checks permissions of the caller on this proxy.
    /// If CanSend returns true then a call to `Execute` with the same message,
    /// before any further state changes, should also succeed.
    CanSend {
        msgs: Vec<CosmosMsg<T>>,
        sig: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: HumanAddr,
    pub public_key: Vec<u8>,
    pub crypto_contract_addr: HumanAddr,
    pub nonce: u64, 
    pub mutable: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CanSendResponse {
    pub can_send: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ContractAddressResponse {
    pub contract_addr: HumanAddr,
}
