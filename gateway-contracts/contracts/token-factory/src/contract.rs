use cosmwasm_std::{
    log, to_binary, Api, Binary, CanonicalAddr, CosmosMsg, Env, Extern, HandleResponse,
    InitResponse, LogAttribute, Querier, StdError, StdResult, Storage, Uint128, WasmMsg,
};

use cw20::MinterResponse;

use axelar_gateway::hook::InitHook;
use axelar_gateway::token::InitMsg as TokenInitMsg;
use axelar_gateway::token_factory::{ConfigResponse, HandleMsg, InitMsg, QueryMsg};

use crate::state::{config_read, config_store, token_address_read, token_address_store, Config};

pub static ATTR_NEW_OWNER: &str = "new_owner";
pub static ATTR_PREV_OWNER: &str = "previous_owner";
pub static ACTION_OWNERSHIP: &str = "ownership";
pub static ACTION_DEPLOY: &str = "deploy_token";
pub static ATTR_SYMBOl: &str = "symbol";

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    let config = Config {
        owner: deps.api.canonical_address(&env.message.sender)?,
        token_code_id: msg.token_code_id,
    };
    config_store(&mut deps.storage, &config)?;

    let mut messages: Vec<CosmosMsg> = vec![];
    if let Some(hook) = msg.init_hook {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: hook.contract_addr,
            msg: hook.msg,
            send: vec![],
        }));
    }

    Ok(InitResponse {
        log: vec![
            log("action", ACTION_OWNERSHIP),
            log(ATTR_PREV_OWNER, "0"),
            log(ATTR_NEW_OWNER, &env.message.sender),
        ],
        messages,
    })
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::DeployToken {
            name,
            symbol,
            decimals,
            cap,
        } => try_deploy_token(deps, env, name, symbol, decimals, cap),
        HandleMsg::Register { symbol } => Ok(HandleResponse::default()),
        HandleMsg::Withdraw { symbol, address } => Ok(HandleResponse::default()),
    }
}

pub fn must_be_owner<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    if deps.api.canonical_address(&env.message.sender)? != config_read(&deps.storage)?.owner {
        return Err(StdError::unauthorized());
    }

    Ok(())
}

pub fn try_deploy_token<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    name: String,
    symbol: String,
    decimals: u8,
    cap: Uint128,
) -> StdResult<HandleResponse> {
    must_be_owner(&deps, &env)?;
    let config = config_read(&deps.storage)?;

    if token_address_read(&deps.storage, &symbol).is_ok() {
        return Err(StdError::generic_err("token already exists"));
    }

    // mark intent to register token address post-initialization
    token_address_store(&mut deps.storage, &symbol, &CanonicalAddr::default())?;

    let messages: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
        code_id: config.token_code_id,
        send: vec![],
        label: Some(name.clone()),
        msg: to_binary(&TokenInitMsg {
            name: name,
            symbol: symbol.clone(),
            decimals: decimals,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                minter: env.message.sender.clone(),
                cap: Some(cap),
            }),
            init_hook: Some(InitHook {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::Register {
                    symbol: symbol.clone(),
                })?,
            }),
        })?,
    })];

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", ACTION_DEPLOY),
            log("symbol", symbol),
        ],
        data: None,
    })
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
    }
}

fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let config = config_read(&deps.storage)?;
    Ok(ConfigResponse {
        owner: deps.api.human_address(&config.owner)?,
        token_code_id: config.token_code_id,
    })
}
