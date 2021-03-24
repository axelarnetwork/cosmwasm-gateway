use crate::{contract::*, state::*};
use axelar_gateway::token::InitMsg as TokenInitMsg;
use axelar_gateway::{
    hook::InitHook,
    token_factory::{ConfigResponse, HandleMsg, InitMsg, QueryMsg},
};
use cosmwasm_std::{
    coins, from_binary, log,
    testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR},
    to_binary, Api, CanonicalAddr, CosmosMsg, HumanAddr, StdError, Uint128, WasmMsg,
};
use cw20::MinterResponse;

#[test]
fn initialization() {
    let mut deps = mock_dependencies(20, &[]);
    let env = mock_env("master_address", &[]);

    let token_code_id: u64 = 1000;
    let init_msg = InitMsg {
        owner: env.message.sender.clone(),
        token_code_id: 1000,
        init_hook: None,
    };

    let sender = env.message.sender.clone();
    let res = init(&mut deps, env, init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    let res = query(&deps, QueryMsg::GetConfig {}).unwrap();
    let value: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!(sender, value.owner);
    assert_eq!(token_code_id, value.token_code_id);
}

#[test]
fn authorization() {
    let mut deps = mock_dependencies(20, &[]);
    let env = mock_env("master_address", &[]);

    let init_msg = InitMsg {
        owner: env.message.sender.clone(),
        token_code_id: 1000,
        init_hook: None,
    };
    let _res = init(&mut deps, env.clone(), init_msg).unwrap();

    let unauth_env = mock_env("anyone", &coins(2, "token"));
    let res = must_be_owner(&deps, &unauth_env.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }

    let res = must_be_owner(&deps, &env.clone());
    match res {
        Err(StdError::Unauthorized { .. }) => panic!("Owner should be authorized"),
        _ => {}
    }
}

#[test]
fn deploy_token() {
    let mut deps = mock_dependencies(20, &[]);

    let token_code_id = 1000u64;
    let env = mock_env("master_address", &[]);
    let init_msg = InitMsg {
        owner: env.message.sender.clone(),
        token_code_id: token_code_id.clone(),
        init_hook: None,
    };

    let _res = init(&mut deps, env.clone(), init_msg).unwrap();

    let symbol = String::from("satoshi");
    let name = String::from("Satoshi");
    let cap = Uint128::from(10000u128);

    let msg = HandleMsg::DeployToken {
        name: name.clone(),
        symbol: symbol.clone(),
        decimals: 8,
        cap: cap.clone(),
    };

    // only owner can deploy new token contracts
    let anon_env = mock_env("addr0000", &[]);
    match handle(&mut deps, anon_env, msg.clone()) {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }

    let res = handle(&mut deps, env.clone(), msg).unwrap();
    assert_eq!(
        res.log,
        vec![log("action", ACTION_DEPLOY), log(ATTR_SYMBOl, &symbol)]
    );

    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Instantiate {
            code_id: token_code_id,
            send: vec![],
            label: Some(name.clone()),
            msg: to_binary(&TokenInitMsg {
                name: name.clone(),
                symbol: symbol.clone(),
                decimals: 8,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.message.sender.clone(),
                    cap: Some(cap.clone()),
                }),
                init_hook: Some(InitHook {
                    contract_addr: HumanAddr::from(MOCK_CONTRACT_ADDR),
                    msg: to_binary(&HandleMsg::Register {
                        symbol: symbol.clone()
                    })
                    .unwrap(),
                })
            })
            .unwrap(),
        })]
    );

    let token_addr = token_address_read(&deps.storage, &symbol).unwrap();
    assert_eq!(token_addr, CanonicalAddr::default(), "deploy token intent not stored");
}