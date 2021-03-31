use std::fmt;

use cosmwasm_std::{
    log, to_binary, to_vec, Api, Binary, CanonicalAddr, CosmosMsg, Empty, Env, Extern,
    HandleResponse, HumanAddr, InitResponse, Querier, QueryResponse, StdError, StdResult, Storage,
    WasmQuery,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{read_config, store_config, Config};

use axelar_gateway::crypto::{InitMsg as CryptoInitMsg, VerifyResponse as CryptoVerifyResponse, QueryMsg as CryptoQueryMsg};
use axelar_gateway::gateway::{CanSendResponse, ConfigResponse, HandleMsg, InitMsg, QueryMsg};
use sha3::{Digest, Keccak256};

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
    cfg.update_owner(deps.api.canonical_address(&msg.owner)?, msg.public_key);
    store_config(&mut deps.storage, &cfg)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::ExecuteSigned { msgs, sig } => handle_execute_signed(deps, env, msgs, sig),
        HandleMsg::Execute { msgs } => handle_execute(deps, env, msgs),
        HandleMsg::UpdateOwner { owner, public_key } => {
            handle_update_owner(deps, env, owner, public_key)
        }
        HandleMsg::Freeze {} => handle_freeze(deps, env),
    }
}

pub fn handle_update_owner<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: HumanAddr,
    public_key: Vec<u8>,
) -> StdResult<HandleResponse> {
    must_be_owner(&deps, &env)?;
    must_not_be_frozen(&deps, &env)?;
    let mut cfg = read_config(&deps.storage)?;

    let owner = deps.api.canonical_address(&owner)?;
    cfg.update_owner(owner, public_key)?;
    store_config(&mut deps.storage, &cfg)?;

    let mut res = HandleResponse::default();
    res.log = vec![log("action", "update_owner")];
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

    if deps.api.canonical_address(&env.message.sender)? != read_config(&deps.storage)?.owner {
        return Ok(());
    }

    Err(StdError::unauthorized())
}

// execute messages authorized with the admin's signature
pub fn handle_execute_signed<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msgs: Vec<CosmosMsg<T>>,
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
) -> StdResult<HandleResponse<T>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    must_be_owner(&deps, &env)?;

    let mut cfg = read_config(&deps.storage)?;
    cfg.increment_nonce();
    let mut res = HandleResponse::default();
    res.messages = msgs;
    res.log = vec![log("action", "execute")];
    Ok(res)
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
    where T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
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

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{CosmosMsg, StdError, WasmMsg, coins, from_binary, testing::{MockApi, MockQuerier, MockStorage}};
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        HumanAddr,
    };
    use crypto_verify::contract as crypto_contract;
    use cw1_whitelist::msg::AdminListResponse;
    use k256::{
        EncodedPoint,
        ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey},
        elliptic_curve::sec1::ToEncodedPoint,
    };

    use axelar_gateway::crypto::{InitMsg as CryptoInitMsg};
    use axelar_gateway::gateway::{CanSendResponse, ConfigResponse, HandleMsg, InitMsg, QueryMsg};
    use rand_core::OsRng;
    use sha3::{Digest, Keccak256};

    const USE_POINT_COMPRESSION: bool = true;
    const CANONICAL_LENGTH: usize = 20;


    fn setup_gateway(mut crypto_addr: HumanAddr) -> (Extern<MockStorage, MockApi, MockQuerier>, Env, HumanAddr, EncodedPoint, SigningKey) {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        let priv_key = SigningKey::random(&mut OsRng); // Serialize with `::to_bytes()`
        let pub_key = VerifyingKey::from(&priv_key).to_encoded_point(USE_POINT_COMPRESSION);
        let axelar = HumanAddr::from("axelar");
        if crypto_addr.len() == 0 {
            crypto_addr = HumanAddr::from("crypto_contract");
        }

        // instantiate the contract
        let msg = InitMsg {
            owner: axelar.clone(),
            public_key: Vec::<u8>::from(pub_key.as_bytes()),
            crypto_contract_addr: crypto_addr.clone(),
        };

        let env = mock_env(axelar.clone(), &[]);
        let res = init(&mut deps, env.clone(), msg).unwrap();

        // ensure expected config
        let expected = ConfigResponse {
            owner: axelar.clone(),
            public_key: Vec::<u8>::from(pub_key.as_bytes()),
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
        let res = crypto_contract::init(&mut deps, mock_env(HumanAddr::from("addr01"), &[]), CryptoInitMsg {}).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    #[test]
    fn initialization() {
        setup_gateway(HumanAddr::default());
    }

    #[test]
    fn execute_signed() {
        let crypto_deps = setup_crypto();
        // todo: how to simulate contract address in env?
        let (mut deps, gateway_env, owner, pub_key, priv_key) = setup_gateway(HumanAddr::default());
        println!("gateway_env {}", gateway_env.contract.address);

        let nonce = 0u64;

        let exec_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: gateway_env.contract.address,
            msg: to_binary(&HandleMsg::<Empty>::Freeze{}).unwrap(),
            send: vec![],
        });

        let messages = vec![exec_msg.clone(),exec_msg.clone(),exec_msg.clone()];

        let digest = digest_message_batch(nonce, &messages).unwrap();
        let sig: Signature = priv_key.sign(digest.as_slice());

        let msg = HandleMsg::ExecuteSigned {
            msgs: messages,
            sig: Vec::<u8>::from(sig.as_ref()),
        };

        let env = mock_env(HumanAddr::from("anyone"), &[]);
        let rest = handle(&mut deps, env, msg).unwrap();

    }
}
