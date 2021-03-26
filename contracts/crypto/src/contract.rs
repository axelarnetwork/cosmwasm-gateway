use cosmwasm_std::{Api, Binary, Extern, Env, InitResponse, Querier, StdError, StdResult, Storage, Uint128, to_binary};
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use std::ops::Deref;

use cosmwasm_crypto::{secp256k1_recover_pubkey,secp256k1_verify};

use crate::msg::{
    list_verifications, InitMsg, ListVerificationsResponse, QueryMsg, VerifyResponse,
};

pub const VERSION: &str = "crypto-verify-v2";

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::VerifyCosmosSignature {
            message,
            signature,
            public_key,
        } => to_binary(&query_verify_cosmos(
            deps,
            message.as_slice(),
            signature.as_slice(),
            public_key.as_slice(),
        )?),
        QueryMsg::ListVerificationSchemes {} => to_binary(&query_list_verifications(deps)?),
    }
}

pub fn query_verify_cosmos<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
    message: &[u8],
    signature: &[u8],
    public_key: &[u8],
) -> StdResult<VerifyResponse> {
    // Hashing
    let hash = Sha256::digest(message);

    // Verification
    let result = secp256k1_verify(hash.as_ref(), signature, public_key);
    match result {
        Ok(verifies) => Ok(VerifyResponse { verifies }),
        Err(err) => Err(StdError::generic_err(err.to_string())),
    }
}

pub fn query_list_verifications<S: Storage, A: Api, Q: Querier> (
deps: &Extern<S, A, Q>
) -> StdResult<ListVerificationsResponse> {
    let verification_schemes: Vec<_> = list_verifications(deps);
    Ok(ListVerificationsResponse {
        verification_schemes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{HumanAddr, from_binary, testing::{
        mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage,
    }};
    use cosmwasm_std::{
        from_slice, Binary, StdError
    };
    use cosmwasm_crypto::{CryptoError};
    use hex_literal::hex;
    use k256::{
        ecdsa::{SigningKey, Signature, signature::Signer},
        SecretKey,
    };
    use rand_core::SeedableRng;

    const CREATOR: &str = "creator";

    const SECP256K1_MESSAGE_HEX: &str = "5c868fedb8026979ebd26f1ba07c27eedf4ff6d10443505a96ecaf21ba8c4f0937b3cd23ffdc3dd429d4cd1905fb8dbcceeff1350020e18b58d2ba70887baa3a9b783ad30d3fbf210331cdd7df8d77defa398cdacdfc2e359c7ba4cae46bb74401deb417f8b912a1aa966aeeba9c39c7dd22479ae2b30719dca2f2206c5eb4b7";
    const SECP256K1_SIGNATURE_HEX: &str = "207082eb2c3dfa0b454e0906051270ba4074ac93760ba9e7110cd9471475111151eb0dbbc9920e72146fb564f99d039802bf6ef2561446eb126ef364d21ee9c4";
    const SECP256K1_PUBLIC_KEY_HEX: &str = "04051c1ee2190ecfb174bfe4f90763f2b4ff7517b70a2aec1876ebcfd644c4633fb03f3cfbd94b1f376e34592d9d41ccaf640bb751b00a1fadeb0c01157769eb73";

    /* #[test]
    fn create_secp256k1_sig () {
        // Signing
        let msg = to_binary(&InitMsg{});

        let signing_key = SigningKey::random(&mut SeedableRng::seed_from_u64(100u64)); // Serialize with `::to_bytes()`
        let message = b"ECDSA proves knowledge of a secret number in the context of a single message";

        // Note: the signature type must be annotated or otherwise inferrable as
        // `Signer` has many impls of the `Signer` trait (for both regular and
        // recoverable signature types).
        let signature: Signature = signing_key.sign(message);

        // Verification
        use k256::{EncodedPoint, ecdsa::{VerifyingKey, signature::Verifier}};

        let verify_key = VerifyingKey::from(&signing_key); // Serialize with `::to_encoded_point()`
        assert!(verify_key.verify(message, &signature).is_ok());

    } */

    fn setup() -> Extern<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies(20, &[]);
        let msg = InitMsg {};
        let res = init(&mut deps, mock_env(HumanAddr::from("addr01"), &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn instantiate_works() {
        setup();
    }

    #[test]
    fn cosmos_signature_verify_works() {
        let deps = setup();

        let message = hex::decode(SECP256K1_MESSAGE_HEX).unwrap();
        let signature = hex::decode(SECP256K1_SIGNATURE_HEX).unwrap();
        let public_key = hex::decode(SECP256K1_PUBLIC_KEY_HEX).unwrap();

        let verify_msg = QueryMsg::VerifyCosmosSignature {
            message: Binary(message),
            signature: Binary(signature),
            public_key: Binary(public_key),
        };

        let raw = query(&deps, verify_msg).unwrap();
        let res: VerifyResponse = from_binary(&raw).unwrap();

        assert_eq!(res, VerifyResponse { verifies: true });
    }

    #[test]
    fn cosmos_signature_verify_fails() {
        let deps = setup();

        let mut message = hex::decode(SECP256K1_MESSAGE_HEX).unwrap();
        // alter message
        message[0] ^= 0x01;
        let signature = hex::decode(SECP256K1_SIGNATURE_HEX).unwrap();
        let public_key = hex::decode(SECP256K1_PUBLIC_KEY_HEX).unwrap();

        let verify_msg = QueryMsg::VerifyCosmosSignature {
            message: Binary(message),
            signature: Binary(signature),
            public_key: Binary(public_key),
        };

        let raw = query(&deps, verify_msg).unwrap();
        let res: VerifyResponse = from_binary(&raw).unwrap();

        assert_eq!(res, VerifyResponse { verifies: false });
    }

    #[test]
    fn cosmos_signature_verify_errors() {
        let deps = setup();

        let message = hex::decode(SECP256K1_MESSAGE_HEX).unwrap();
        let signature = hex::decode(SECP256K1_SIGNATURE_HEX).unwrap();
        let public_key = vec![];

        let verify_msg = QueryMsg::VerifyCosmosSignature {
            message: Binary(message),
            signature: Binary(signature),
            public_key: Binary(public_key),
        };

        let res = query(&deps, verify_msg);
        /* assert!(res.is_err());
        assert_eq!(
            res.unwrap_err(),
            StdError::VerificationErr {
                source: VerificationError::InvalidPubkeyFormat
            }
        ) */
    }

    #[test]
    fn list_signatures_works() {
        let deps = setup();

        let query_msg = QueryMsg::ListVerificationSchemes {};

        let raw = query(&deps, query_msg).unwrap();
        let res: ListVerificationsResponse = from_binary(&raw).unwrap();

        assert_eq!(
            res,
            ListVerificationsResponse {
                verification_schemes: vec![
                    "secp256k1".into(),
                ]
            }
        );
    }
}
