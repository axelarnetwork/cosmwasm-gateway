#![allow(clippy::field_reassign_with_default)] // see https://github.com/CosmWasm/cosmwasm/issues/685

use cosmwasm_std::{Api, Binary, Extern, Querier, Storage};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HandleMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Cosmos format (secp256k1 verification scheme).
    VerifyCosmosSignature {
        /// Message to verify.
        message: Binary,
        /// Serialized signature. Cosmos format (64 bytes).
        signature: Binary,
        /// Serialized compressed (33 bytes) or uncompressed (65 bytes) public key.
        public_key: Binary,
    },

    /// Cosmos Address Recovery
    RecoverCosmosAddress {
        /// Message to verify
        message: String,
        /// Serialized signature. Fixed length format (64 bytes `r` and `s` plus the one byte `v`).
        signature: Binary,
        /// Signer address.
        /// This is matched case insensitive, so you can provide checksummed and non-checksummed addresses. Checksums are not validated.
        signer_address: String,
    },
    /// Returns a list of supported verification schemes.
    /// No pagination - this is a short list.
    ListVerificationSchemes {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct VerifyResponse {
    pub verifies: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListVerificationsResponse {
    pub verification_schemes: Vec<String>,
}
