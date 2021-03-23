use cosmwasm_std::{
    log, to_binary, Api, Binary, CosmosMsg, Env, Extern, HandleResponse, InitResponse,
    LogAttribute, Querier, StdError, StdResult, Storage, WasmMsg,
};

use crate::state::{config_read, config_store, Config};

use axelar_gateway::common::log_attribute;
use axelar_gateway::token_factory::{HandleMsg, InitMsg, QueryMsg, ConfigResponse};

pub static ATTR_NEW_OWNER: &str = "new_owner";
pub static ATTR_PREV_OWNER: &str = "previous_owner";
pub static LOG_KEY_OWNERSHIP: &str = "ownership_transferred";

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
            log_attribute(LOG_KEY_OWNERSHIP, ATTR_PREV_OWNER, "0"),
            log_attribute(LOG_KEY_OWNERSHIP, ATTR_NEW_OWNER, env.message.sender),
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
    capacity: u128,
) -> StdResult<HandleResponse> {
    must_be_owner(&deps, &env)?;
    let config = config_read(&deps.storage)?;

    /*
    // 1. create the init message
    let mut messages: Vec<CosmosMsg> = vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
        code_id: config.pair_code_id,
        send: vec![],
        label: None,
        msg: to_binary(&PairInitMsg {
            asset_infos: asset_infos.clone(),
            token_code_id: config.token_code_id,
            init_hook: Some(InitHook {
                contract_addr: env.contract.address,
                msg: to_binary(&HandleMsg::Register {
                    asset_infos: asset_infos.clone(),
                })?,
            }),
        })?,
    })];

    if let Some(hook) = init_hook {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: hook.contract_addr,
            msg: hook.msg,
            send: vec![],
        }));
    }

    Ok(HandleResponse {
        messages,
        log: vec![
            log("action", "create_pair"),
            log("pair", format!("{}-{}", asset_infos[0], asset_infos[1])),
        ],
        data: None,
    })
    */

    Ok(HandleResponse::default())
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
        owner: config.owner,
        token_code_id: config.token_code_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axelar_gateway::token_factory::{HandleMsg, InitMsg, QueryMsg, ConfigResponse};

    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, HumanAddr, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let master_address = HumanAddr::from("master_address");
        let canon_master_address = deps.api.canonical_address(&master_address).unwrap();

        let env = mock_env(master_address, &coins(1000, "earth"));

        let token_code_id: u64 = 1000;
        let init_msg = InitMsg {
            owner: canon_master_address.clone(),
            token_code_id: 1000,
            init_hook: None,
        };

        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(&deps, QueryMsg::GetConfig {}).unwrap();
        let value: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!(canon_master_address, value.owner);
        assert_eq!(token_code_id, value.token_code_id);
    }

    #[test]
    fn authorization() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let master_address = HumanAddr::from("master_address");
        let env = mock_env(master_address.clone(), &coins(2, "token"));
        let _res = init(&mut deps, env, InitMsg {}).unwrap();

        let unauth_env = mock_env("anyone", &coins(2, "token"));

        let res = must_be_owner(&deps, &unauth_env.clone());
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }

        let env = mock_env(master_address.clone(), &coins(2, "token"));
        let res = must_be_owner(&deps, &env.clone());
        match res {
            Err(StdError::Unauthorized { .. }) => panic!("Owner should be authorized"),
            _ => {}
        }
    }
}
