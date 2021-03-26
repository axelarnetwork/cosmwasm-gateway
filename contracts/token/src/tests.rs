use crate::{contract::*};
use cosmwasm_std::{Uint128, log, testing::{mock_dependencies, mock_env}};
use cosmwasm_std::{coins, from_binary, HumanAddr, StdError};
use cw20::{MinterResponse, BalanceResponse};

use axelar_gateway::{hook::InitHook, token::{HandleMsg, InitMsg, QueryMsg}};

#[test]
fn initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let proxy = HumanAddr::from("master_address");
    let payer = HumanAddr::from("payer");
    let env = mock_env(payer.clone(), &[]);

    let cap: u128 = 1000000;
    let cap = Uint128::from(cap);
    let init_msg = InitMsg {
        name: "axelar".to_string(),
        symbol: "XLR".to_string(),
        decimals: 8,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: proxy.clone(),
            cap: Some(cap),
        }),
        init_hook: None,
    };

    let res = init(&mut deps, env.clone(), init_msg).unwrap();
    assert_eq!(0, res.messages.len());

    // init message without minter cap
    let init_msg = InitMsg {
        name: "axelar".to_string(),
        symbol: "XLR".to_string(),
        decimals: 8,
        initial_balances: vec![],
        mint: None,
        init_hook: None,
    };
    let res = init(&mut deps, env.clone(), init_msg);
    match res {
        Err(StdError::GenericErr {msg, backtrace}) => assert_eq!(MINTER_ERROR, msg),
        _ => panic!("Must return minter error"),
    }
}

#[test]
fn mint() {
    let mut deps = mock_dependencies(20, &[]);
    let proxy = HumanAddr::from("master_address");
    let payer = HumanAddr::from("payer");

    let payer_env = mock_env(payer.clone(), &coins(1000, "earth"));

    let cap: u128 = 1000000;
    let cap = Uint128::from(cap);
    let init_msg = InitMsg {
        name: "axelar".to_string(),
        symbol: "XLR".to_string(),
        decimals: 8,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: proxy.clone(),
            cap: Some(cap),
        }),
        init_hook: None,
    };

    let _res = init(&mut deps, payer_env, init_msg).unwrap();

    // mint params
    let recipient = HumanAddr::from("recip001");
    let amount = Uint128::from(100u64); 
    let msg = HandleMsg::Mint {
        recipient: recipient.clone(),
        amount: amount.clone(),
    };

    // authorized minter
    let proxy_env = mock_env(proxy.clone(), &[]);
    let _res = handle(&mut deps, proxy_env, msg.clone()).unwrap();

    let query_res = query(&deps, QueryMsg::Balance { address: recipient }).unwrap();
    let balance_res: BalanceResponse = from_binary(&query_res).unwrap(); 
    assert_eq!(amount, balance_res.balance);

    // unauthorized minter
    let env = mock_env("anyone000", &[]);
    match handle(&mut deps, env, msg) {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("must return unauthorized error"),
    }
}

#[test]
fn burn() {
    let mut deps = mock_dependencies(20, &[]);
    let proxy = HumanAddr::from("master_address");
    let payer = HumanAddr::from("payer");

    let payer_env = mock_env(payer.clone(), &coins(1000, "earth"));

    let cap: u128 = 1000000;
    let cap = Uint128::from(cap);
    let init_msg = InitMsg {
        name: "axelar".to_string(),
        symbol: "XLR".to_string(),
        decimals: 8,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: proxy.clone(),
            cap: Some(cap),
        }),
        init_hook: None,
    };

    let _res = init(&mut deps, payer_env, init_msg).unwrap();

    let recipient = proxy.clone();
    let amount = Uint128::from(100u64); 

    // mint some tokens to burn
    let proxy_env = mock_env(proxy.clone(), &[]);
    let msg = HandleMsg::Mint {
        recipient: recipient.clone(),
        amount: amount.clone(),
    };
    let _res = handle(&mut deps, proxy_env.clone(), msg).unwrap();

    let msg = HandleMsg::Burn {
        amount: amount.clone(),
    };

    // authorized burner
    let res = handle(&mut deps, proxy_env, msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "burn"),
            log("from", proxy.clone()),
            log("amount", amount),
        ],
    );
    assert_eq!(0, res.messages.len());

    // check balance was updated
    let query_res = query(&deps, QueryMsg::Balance { address: recipient }).unwrap();
    let balance_res: BalanceResponse = from_binary(&query_res).unwrap(); 
    assert_eq!(Uint128::zero(), balance_res.balance);

    // unauthorized burner
    let env = mock_env("anyone000", &[]);
    match handle(&mut deps, env, msg) {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("must return unauthorized error"),
    }
}

#[test]
fn withdraw() {
    let mut deps = mock_dependencies(20, &[]);
    let proxy = HumanAddr::from("master_address");
    let payer = HumanAddr::from("payer");

    let payer_env = mock_env(payer.clone(), &coins(1000, "earth"));

    let cap: u128 = 1000000;
    let cap = Uint128::from(cap);
    let init_msg = InitMsg {
        name: "axelar".to_string(),
        symbol: "XLR".to_string(),
        decimals: 8,
        initial_balances: vec![],
        mint: Some(MinterResponse {
            minter: proxy.clone(),
            cap: Some(cap),
        }),
        init_hook: None,
    };

    let _res = init(&mut deps, payer_env, init_msg).unwrap();

    let user = HumanAddr::from("user001");
    let amount = Uint128::from(100u64); 

    // mint some tokens to withdraw
    let proxy_env = mock_env(proxy.clone(), &[]);
    let msg = HandleMsg::Mint {
        recipient: user.clone(),
        amount: amount.clone(),
    };
    let _res = handle(&mut deps, proxy_env.clone(), msg).unwrap();

    // attempt withdraw
    let cross_chain_addr = HumanAddr::from("tb1qw99lg2um87u0gxx4c8k9f9h8ka0tcjcmjk92np");
    let msg = HandleMsg::Withdraw {
        recipient: cross_chain_addr.clone(),
        amount: amount.clone(),
    };

    let user_env = mock_env(user.clone(), &[]);
    let res = handle(&mut deps, user_env, msg.clone()).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "withdraw"),
            log("from", user.clone()),
            log("to", cross_chain_addr.clone()),
            log("amount", amount),
        ],
    );
    assert_eq!(0, res.messages.len());

    // check balance was updated
    let query_res = query(&deps, QueryMsg::Balance { address: user }).unwrap();
    let balance_res: BalanceResponse = from_binary(&query_res).unwrap(); 
    assert_eq!(Uint128::zero(), balance_res.balance);
}
