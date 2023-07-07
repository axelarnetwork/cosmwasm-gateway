use cosmwasm_std::{
    to_binary, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, InitResponse, Querier,
    StdError, StdResult, Storage, Uint128,
};
use sha2::{Digest, Sha256};
use sha3::Keccak256;
use std::ops::Deref;

use cosmwasm_crypto::{secp256k1_recover_pubkey, secp256k1_verify};

use axelar_gateway_contracts::{
    crypto::{InitMsg, ListVerificationsResponse, QueryMsg, VerifyResponse},
    gateway::HandleMsg,
};

pub const VERSION: &str = "crypto-verify-v2";

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    Err(StdError::not_found("no handlers exist"))
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
        QueryMsg::RecoverCosmosAddress { .. } => Ok(Binary::default()),
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

pub fn query_list_verifications<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ListVerificationsResponse> {
    let verification_schemes: Vec<_> = list_verifications(deps);
    Ok(ListVerificationsResponse {
        verification_schemes,
    })
}

pub(crate) fn list_verifications<S: Storage, A: Api, Q: Querier>(
    _deps: &Extern<S, A, Q>,
) -> Vec<String> {
    vec!["secp256k1".into()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_crypto::CryptoError;
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage},
        HumanAddr,
    };
    use cosmwasm_std::{from_slice, Binary, StdError};
    use hex_literal::hex;
    use k256::{
        ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey},
        elliptic_curve::sec1::ToEncodedPoint,
        CompressedPoint, EncodedPoint, PublicKey, SecretKey,
    };
    use rand_core::OsRng;
    use sha2::{Digest, Sha256};

    const CREATOR: &str = "creator";

    const SECP256K1_MESSAGE_HEX: &str = "5c868fedb8026979ebd26f1ba07c27eedf4ff6d10443505a96ecaf21ba8c4f0937b3cd23ffdc3dd429d4cd1905fb8dbcceeff1350020e18b58d2ba70887baa3a9b783ad30d3fbf210331cdd7df8d77defa398cdacdfc2e359c7ba4cae46bb74401deb417f8b912a1aa966aeeba9c39c7dd22479ae2b30719dca2f2206c5eb4b7";
    const SECP256K1_SIGNATURE_HEX: &str = "207082eb2c3dfa0b454e0906051270ba4074ac93760ba9e7110cd9471475111151eb0dbbc9920e72146fb564f99d039802bf6ef2561446eb126ef364d21ee9c4";
    const SECP256K1_PUBLIC_KEY_HEX: &str = "04051c1ee2190ecfb174bfe4f90763f2b4ff7517b70a2aec1876ebcfd644c4633fb03f3cfbd94b1f376e34592d9d41ccaf640bb751b00a1fadeb0c01157769eb73";

    #[test]
    fn cosmos_verify_message_batch() {
        // Signing
        let signing_key = SigningKey::random(&mut OsRng); // Serialize with `::to_bytes()`
        let pub_key = VerifyingKey::from(&signing_key); // Serialize with `::to_encoded_point()`

        let msg = InitMsg {};
        let messages = vec![msg.clone(), msg.clone(), msg.clone()];
        let digest = Sha256::digest(to_binary(&messages).unwrap().as_slice());

        let signature: Signature = signing_key.sign(&digest);

        let verify_msg = QueryMsg::VerifyCosmosSignature {
            message: Binary::from(digest.as_slice()),
            signature: Binary::from(signature.as_ref()),
            public_key: Binary::from(pub_key.to_encoded_point(false).as_bytes()),
        };

        let deps = setup();
        let raw = query(&deps, verify_msg).unwrap();
        let res: VerifyResponse = from_binary(&raw).unwrap();
        assert_eq!(res, VerifyResponse { verifies: true });
    }

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
        assert!(res.is_err());
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
                verification_schemes: vec!["secp256k1".into(),]
            }
        );
    }
}
