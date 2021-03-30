use core::fmt;

use cosmwasm_std::{Api, Binary, QueryResponse, CosmosMsg, Env, Extern, Empty, HandleResponse, HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, WasmQuery, log, to_binary, to_vec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::state::{store_config, read_config, Config};

use axelar_gateway::{gateway::{HandleMsg, InitMsg, QueryMsg, ConfigResponse, CanSendResponse}} ;
use axelar_gateway::crypto;
use sha3::{Keccak256, Digest};


pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
    ) -> StdResult<InitResponse> {

    let cfg = Config {
        owner: deps.api.canonical_address(&msg.owner)?,
        public_key: msg.public_key,
        crypto_contract_addr: deps.api.canonical_address(&msg.crypto_contract_addr)?,
        nonce: 0,
        mutable: true,
    };
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

    // todo: recover signer canon address from hash of packed messages
    if !verify_signed_by_owner(&deps, msgs, sig)? {
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
    msgs: Vec<CosmosMsg<T>>,
    sig: Vec<u8>,
    ) -> StdResult<bool> 
where
T: Clone + fmt::Debug + PartialEq + JsonSchema + Serialize,
{
    let cfg = read_config(&deps.storage)?;

    // serialize cosmos messages into base64 encoded json
    let msgs: Result<Vec<_>, StdResult<bool>> = msgs
        .into_iter()
        .map(|msg| to_vec(&msg))
        .collect();

    let serials = match msgs {
        Ok(slice) => slice,
        Err(err) => return err,
    };

    let mut bytes: Vec<u8> = serials.concat();

    // append the nonce and calculate the message digest
    bytes.extend_from_slice(&cfg.nonce.to_be_bytes());
    let digest = Keccak256::digest(bytes.as_slice()).as_slice();

    let verify_msg = crypto::QueryMsg::VerifyCosmosSignature{
        message: Binary::from(digest),
        signature: Binary::from(sig),
        public_key: Binary::from(cfg.public_key),
    };

    let res: crypto::VerifyResponse = deps.querier.query(&cosmwasm_std::QueryRequest::Wasm(WasmQuery::Smart{
        contract_addr: deps.api.human_address(&cfg.crypto_contract_addr)?,
        msg: to_binary(&verify_msg)?,
    }))?;

    Ok(res.verifies)
}

pub fn handle_freeze<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    ) -> StdResult<HandleResponse> {
    must_be_owner(&deps, &env)?;
    must_not_be_frozen(&deps, &env)?;

    let cfg = read_config(&deps.storage)?;
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
        QueryMsg::CanSend { sig, msgs } => to_binary(&query_can_send(deps, msgs, sig )?),
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
        can_send: verify_signed_by_owner(&deps, msgs, sig)?,
    })
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{HumanAddr, testing::{mock_dependencies, mock_env}};
    use cosmwasm_std::{coins, from_binary, StdError};
    use cw1_whitelist::msg::AdminListResponse;
    use k256::{
        ecdsa::{signature::Signer, signature::Verifier, Signature, SigningKey, VerifyingKey},
        elliptic_curve::sec1::ToEncodedPoint,
    };

    use axelar_gateway::{gateway::{HandleMsg, InitMsg, QueryMsg, ConfigResponse, CanSendResponse}} ;
    use rand_core::OsRng;
    use sha3::{Keccak256, Digest};

    const USE_POINT_COMPRESSION: bool = true;

    #[test]
    fn initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let signing_key = SigningKey::random(&mut OsRng); // Serialize with `::to_bytes()`
        let pub_key = VerifyingKey::from(&signing_key).to_encoded_point(USE_POINT_COMPRESSION).as_bytes(); // Serialize with `::to_encoded_point()`

        let axelar = HumanAddr::from("axelar");
        let anyone = HumanAddr::from("anyone");
        let crypto_addr = HumanAddr::from("crypto_contract");

        // instantiate the contract
        let msg = InitMsg {
            owner: axelar.clone(),
            public_key: Vec::<u8>::from(pub_key),
            crypto_contract_addr: crypto_addr.clone(),
        };

        let env = mock_env(axelar.clone(), &[]);
        let res = init(&mut deps, env, msg).unwrap();

        // ensure expected config
        let expected = ConfigResponse {
            owner: axelar.clone(),
            public_key: Vec::<u8>::from(pub_key),
            crypto_contract_addr: crypto_addr.clone(),
            nonce: 0u64,
            mutable: true,
        };
        assert_eq!(query_config(&deps).unwrap(), expected);

        assert_eq!(0, res.messages.len());
    }
}
