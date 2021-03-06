use std::fmt;

use cosmwasm_std::{
    from_binary, log, to_binary, to_vec, Api, Binary, CanonicalAddr, CosmosMsg, Empty, Env, Extern,
    HandleResponse, HumanAddr, InitResponse, Querier, QueryResponse, StdError, StdResult, Storage,
    WasmQuery,
};
use k256::{ecdsa::VerifyingKey, CompressedPoint};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{
    read_config, read_contract_address, store_config, store_contract_address,
    verifying_key_from_base64_str, Config,
};

use axelar_gateway_contracts::crypto::{
    InitMsg as CryptoInitMsg, QueryMsg as CryptoQueryMsg, VerifyResponse as CryptoVerifyResponse,
};
use axelar_gateway_contracts::gateway::{
    CanSendResponse, ConfigResponse, HandleMsg, InitMsg, QueryMsg, ContractAddressResponse
};
use sha3::{Digest, Keccak256};

pub static ATTR_NEW_OWNER: &str = "new_owner";
pub static ATTR_PREV_OWNER: &str = "previous_owner";
pub static ACTION_OWNERSHIP: &str = "ownership";

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let mut cfg = Config {
        crypto_contract_addr: deps.api.canonical_address(&msg.crypto_contract_addr)?,
        nonce: 0,
        mutable: true,
        owner: CanonicalAddr::default(),
        public_key: vec![],
    };
    cfg.update_owner(deps.api.canonical_address(&msg.owner)?, &msg.public_key)?;

    store_config(&mut deps.storage, &cfg)?;
    Ok(InitResponse {
        log: vec![
            log("action", ACTION_OWNERSHIP),
            log(ATTR_PREV_OWNER, "0"),
            log(ATTR_NEW_OWNER, msg.owner),
        ],
        messages: vec![],
    })
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::ExecuteSigned {
            sig,
            msgs,
            register,
        } => handle_execute_signed(deps, env, msgs, register, sig),
        HandleMsg::Execute { msgs, register } => handle_execute(deps, env, msgs, register),
        HandleMsg::Register { name } => handle_register_contract(deps, env, name),
        HandleMsg::UpdateOwner { owner, public_key } => {
            handle_update_owner(deps, env, owner, public_key)
        }
        HandleMsg::Freeze {} => handle_freeze(deps, env),
    }
}

pub fn handle_register_contract<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
) -> StdResult<HandleResponse> {
    let contract_addr = read_contract_address(&deps.storage, &name)?; // fails if name not deployed
    if contract_addr != CanonicalAddr::default() {
        return Err(StdError::generic_err("contract already registered"));
    }

    let contract_addr = deps.api.canonical_address(&env.message.sender)?;

    // mark intent to register contract address post-initialization
    store_contract_address(&mut deps.storage, &name, &contract_addr)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "register"),
            log("contract_addr", contract_addr),
        ],
        data: None,
    })
}

pub fn handle_update_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: HumanAddr,
    public_key: String,
) -> StdResult<HandleResponse> {
    must_be_owner(&deps, &env)?;
    must_not_be_frozen(&deps, &env)?;
    let mut cfg = read_config(&deps.storage)?;
    let prev_owner = deps.api.human_address(&cfg.owner)?;

    cfg.update_owner(deps.api.canonical_address(&owner)?, &public_key)?;
    store_config(&mut deps.storage, &cfg)?;

    let mut res = HandleResponse::default();
    res.log = vec![
        log("action", ACTION_OWNERSHIP),
        log(ATTR_PREV_OWNER, prev_owner),
        log(ATTR_NEW_OWNER, owner),
    ];
    Ok(res)
}

pub fn must_not_be_frozen<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    if read_config(&deps.storage)?.mutable == false {
        return Err(StdError::unauthorized());
    }
    Ok(())
}

pub fn must_be_owner<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    if env.message.sender == env.contract.address {
        // allow the gateway to proxy messages to itself
        return Ok(());
    }

    if deps.api.canonical_address(&env.message.sender)? == read_config(&deps.storage)?.owner {
        return Ok(());
    }

    Err(StdError::unauthorized())
}

// execute messages authorized with the admin's signature
pub fn handle_execute_signed<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msgs: Vec<CosmosMsg<T>>,
    register: Vec<String>,
    sig: Vec<u8>,
) -> StdResult<HandleResponse<T>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    must_not_be_frozen(&deps, &env)?;

    if !verify_signed_by_owner(&deps, &msgs, sig)? {
        Err(StdError::unauthorized())
    } else {
        let mut cfg = read_config(&deps.storage)?;
        cfg.increment_nonce();
        store_config(&mut deps.storage, &cfg)?;

        store_registration_intent(deps, register)?;

        let mut res = HandleResponse::default();
        res.messages = msgs;
        res.log = vec![log("action", "execute")];
        Ok(res)
    }
}
pub fn handle_execute<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msgs: Vec<CosmosMsg<T>>,
    register: Vec<String>,
) -> StdResult<HandleResponse<T>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    must_be_owner(&deps, &env)?;

    store_registration_intent(deps, register)?;

    let mut res = HandleResponse::default();
    res.messages = msgs;

    // todo: log registered names
    res.log = vec![log("action", "execute")];
    Ok(res)
}

pub fn store_registration_intent<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    names: Vec<String>,
) -> StdResult<()> {
    names
        .iter()
        .map(|name| {
            if read_contract_address(&deps.storage, &name).is_ok() {
                return Err(StdError::generic_err("contract already exists"));
            }

            // mark intent to register contract address post-initialization
            store_contract_address(&mut deps.storage, &name, &CanonicalAddr::default())
        })
        .collect::<StdResult<()>>()
}

fn verify_signed_by_owner<S: Storage, A: Api, Q: Querier, T>(
    deps: &Extern<S, A, Q>,
    msgs: &Vec<CosmosMsg<T>>,
    sig: Vec<u8>,
) -> StdResult<bool>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    let cfg = read_config(&deps.storage)?;

    // serialize cosmos messages into base64 encoded json
    let mut bytes = msgs
        .into_iter()
        .map(|msg| to_vec(&msg))
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    // append the nonce and calculate the message digest
    bytes.extend_from_slice(&cfg.nonce.to_be_bytes());
    let digest = Keccak256::digest(bytes.as_slice());

    let verify_msg = CryptoQueryMsg::VerifyCosmosSignature {
        message: Binary::from(digest.as_slice()),
        signature: Binary::from(sig),
        public_key: Binary::from(cfg.public_key),
    };

    let res: CryptoVerifyResponse =
        deps.querier
            .query(&cosmwasm_std::QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: deps.api.human_address(&cfg.crypto_contract_addr)?,
                msg: to_binary(&verify_msg)?,
            }))?;

    Ok(res.verifies)
}

pub fn digest_message_batch<T>(nonce: u64, msgs: &Vec<CosmosMsg<T>>) -> StdResult<Vec<u8>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    // serialize cosmos messages into base64 encoded json
    let mut bytes = msgs
        .into_iter()
        .map(|msg| to_vec(&msg))
        .collect::<Result<Vec<_>, _>>()?
        .concat();

    // append the nonce and calculate the message digest
    bytes.extend_from_slice(&nonce.to_be_bytes());
    Ok(Keccak256::digest(bytes.as_slice()).to_vec())
}

pub fn handle_freeze<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    must_be_owner(&deps, &env)?;
    must_not_be_frozen(&deps, &env)?;

    let mut cfg = read_config(&deps.storage)?;
    cfg.mutable = false;
    store_config(&mut deps.storage, &cfg)?;

    let mut res = HandleResponse::default();
    res.log = vec![log("action", "freeze")];
    Ok(res)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::ContractAddress { name } => to_binary(&query_contract_address(deps, name)?),
        QueryMsg::CanSend { sig, msgs } => to_binary(&query_can_send(deps, msgs, sig)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let cfg = read_config(&deps.storage)?;
    Ok(ConfigResponse {
        owner: deps.api.human_address(&cfg.owner)?,
        public_key: cfg.public_key,
        crypto_contract_addr: deps.api.human_address(&cfg.crypto_contract_addr)?,
        nonce: cfg.nonce,
        mutable: cfg.mutable,
    })
}

fn query_can_send<S: Storage, A: Api, Q: Querier, T>(
    deps: &Extern<S, A, Q>,
    msgs: Vec<CosmosMsg<T>>,
    sig: Vec<u8>,
) -> StdResult<CanSendResponse>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    Ok(CanSendResponse {
        can_send: verify_signed_by_owner(&deps, &msgs, sig)?,
    })
}

fn query_contract_address<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    name: String,
) -> StdResult<ContractAddressResponse> {
    let canon_addr = read_contract_address(&deps.storage, &name)?;

    let contract_address = match canon_addr.len() {
        0 => HumanAddr::default(), // test api will panic if canon_addr = CanonAddr::default()
        _ => deps.api.human_address(&canon_addr)?,
    };

    Ok(ContractAddressResponse {
        contract_addr: contract_address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axelar_crypto::contract as crypto_contract;
    use cosmwasm_std::{
        coins, from_binary,
        testing::{mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage},
        Binary, CosmosMsg, HumanAddr, StdError, WasmMsg,
    };
    use k256::{
        ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey},
        elliptic_curve::sec1::ToEncodedPoint,
        CompressedPoint, EncodedPoint,
    };

    use axelar_gateway_contracts::crypto::{
        InitMsg as CryptoInitMsg, QueryMsg as CryptoQueryMsg,
        VerifyResponse as CryptoVerifyResponse,
    };
    use axelar_gateway_contracts::gateway::{
        CanSendResponse, ConfigResponse, HandleMsg, InitMsg, QueryMsg,
    };
    use rand_core::OsRng;
    use sha3::{Digest, Keccak256};

    use crate::state::{base64_str_from_sec1_bytes, verifying_key_from_base64_str};

    const USE_POINT_COMPRESSION: bool = true;
    const CANONICAL_LENGTH: usize = 20;

    // default localterra public key
    const PUBLIC_KEY_BASE64_COMPRESSED: &str = "";

    fn setup_gateway(
        mut crypto_addr: HumanAddr,
    ) -> (
        Extern<MockStorage, MockApi, MockQuerier>,
        Env,
        HumanAddr,
        CompressedPoint,
        SigningKey,
    ) {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        let priv_key = SigningKey::random(&mut OsRng); // Serialize with `::to_bytes()`
        let verifying_key = VerifyingKey::from(&priv_key);
        let pub_key = verifying_key.to_bytes(); // SEC-1 encoded compressed point

        let axelar = HumanAddr::from("axelar");
        if crypto_addr.len() == 0 {
            crypto_addr = HumanAddr::from("crypto_contract");
        }

        // instantiate the contract
        let msg = InitMsg {
            owner: axelar.clone(),
            public_key: base64_str_from_sec1_bytes(&pub_key),
            crypto_contract_addr: crypto_addr.clone(),
        };

        let env = mock_env(axelar.clone(), &[]);
        let res = init(&mut deps, env.clone(), msg).unwrap();

        // ensure expected config
        let expected = ConfigResponse {
            owner: axelar.clone(),
            public_key: pub_key.to_vec(),
            crypto_contract_addr: crypto_addr.clone(),
            nonce: 0u64,
            mutable: true,
        };
        assert_eq!(query_config(&deps).unwrap(), expected);

        assert_eq!(0, res.messages.len());
        (deps, env, axelar, pub_key, priv_key)
    }

    fn setup_crypto() -> Extern<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);
        let res = crypto_contract::init(
            &mut deps,
            mock_env(HumanAddr::from("addr01"), &[]),
            CryptoInitMsg {},
        )
        .unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn pubkey_format() {
        // This test serves as guide for producing a correctly encoded public key for
        // the InitMsg{} and HandleMsg::UpdateOwner{} messages.

        let priv_key = SigningKey::random(&mut OsRng); // Serialize with `::to_bytes()`
        let verifying_key = VerifyingKey::from(&priv_key);
        let pub_key = verifying_key.to_bytes();

        // check base64 string representation maps correctly to SEC-1 bytes
        let pk_str = base64_str_from_sec1_bytes(&pub_key);
        println!("pk_str {}", pk_str);
        let vk_import = verifying_key_from_base64_str(&pk_str).unwrap();

        // to compressed point
        let imported_pub_key = vk_import.to_bytes();

        // back to base64 str
        let imp_str = base64_str_from_sec1_bytes(&imported_pub_key);
        assert_eq!(imported_pub_key, pub_key);
        assert_eq!(imp_str, pk_str);

        // test key created using terra.js
        /* let verifying_key = verifying_key_from_base64_str(PUBLIC_KEY_BASE64_COMPRESSED).unwrap();
        let imp_str = base64_str_from_sec1_bytes(&verifying_key.to_bytes());
        assert_eq!(PUBLIC_KEY_BASE64_COMPRESSED, imp_str); */
    }

    #[test]
    fn initialization() {
        setup_gateway(HumanAddr::default());
    }

    #[test]
    fn execute_signed() {
        let (mut deps, gateway_env, owner, pub_key, priv_key) = setup_gateway(HumanAddr::default());

        let nonce = 0u64;

        let exec_msg = CosmosMsg::Wasm::<Empty>(WasmMsg::Execute {
            contract_addr: gateway_env.contract.address,
            msg: to_binary(&HandleMsg::<Empty>::Freeze {}).unwrap(),
            send: vec![],
        });

        let messages = vec![exec_msg.clone(), exec_msg.clone(), exec_msg.clone()];

        let digest = digest_message_batch(nonce, &messages).unwrap();
        let sig: Signature = priv_key.sign(digest.as_slice());

        let msg = HandleMsg::ExecuteSigned {
            msgs: messages,
            sig: Vec::<u8>::from(sig.as_ref()),
            register: vec![],
        };

        // simulate query from gateway contract for now
        let verify_msg = CryptoQueryMsg::VerifyCosmosSignature {
            message: Binary::from(digest.as_slice()),
            signature: Binary::from(sig.as_ref()),
            public_key: Binary::from(pub_key.as_ref()),
        };
        let crypto_deps = setup_crypto();
        let raw =
            crypto_contract::query::<MockStorage, MockApi, MockQuerier>(&crypto_deps, verify_msg)
                .unwrap();
        let res: CryptoVerifyResponse = from_binary(&raw).unwrap();
        assert_eq!(res, CryptoVerifyResponse { verifies: true });
        /*
               // todo: how to simulate contract address in env?
               let env = mock_env(HumanAddr::from("anyone"), &[]);
               let rest = handle(&mut deps, env, msg).unwrap();
        */
    }
}
