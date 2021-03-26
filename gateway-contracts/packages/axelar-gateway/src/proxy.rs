use std::fmt;

use cosmwasm_std::{CosmosMsg, Empty, HumanAddr};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cw1_whitelist::{state::AdminList, msg::InitMsg as Cw1InitMsg};

pub type InitMsg = Cw1InitMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    // todo: implement verification
    ExecuteWrapped { msgs: Vec<CosmosMsg<T>>, sig: String },

    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. Every implementation has it's own logic to
    /// determine in
    Execute { msgs: Vec<CosmosMsg<T>> },
    /// Freeze will make a mutable contract immutable, must be called by an admin
    Freeze {},
    /// UpdateAdmins will change the admin set of the contract, must be called by an existing admin,
    /// and only works if the contract is mutable
    UpdateAdmins { admins: Vec<HumanAddr> },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Shows all admins and whether or not it is mutable
    AdminList {},
    /// Checks permissions of the caller on this proxy.
    /// If CanSend returns true then a call to `Execute` with the same message,
    /// before any further state changes, should also succeed.
    CanSend {
        sender: HumanAddr,
        msg: CosmosMsg<T>,
    },
}
