use cosmwasm_std::{
    Api, Binary, CosmosMsg, Env, Extern, HandleResult, InitResponse, MigrateResult, Querier, StdError, StdResult, Storage, WasmMsg,
};

use cw2::set_contract_version;
use cw20_base::contract::{
    create_accounts, handle as cw20_handle, migrate as cw20_migrate, query as cw20_query,
};
use cw20_base::msg::{HandleMsg, MigrateMsg, QueryMsg};
use cw20_base::state::{token_info, MinterData, TokenInfo};

use axelar_gateway::token::InitMsg;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const MINTER_ERROR: &str = "Must provide minter data";

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Check valid token info
    msg.validate()?;

    // Create initial accounts
    let total_supply = create_accounts(deps, &msg.initial_balances)?;

    // Check supply cap
    if let Some(limit) = msg.get_cap() {
        if total_supply > limit {
            return Err(StdError::generic_err("Initial supply greater than cap"));
        }
    }

    // gateway tokens must be initialized with maximum capacity and mint authority
    let mint: Option<MinterData>;
    match msg.mint {
        Some(m) => mint = Some(MinterData {
            minter: deps.api.canonical_address(&m.minter)?,
            cap: m.cap,
        }),
        None => return Err(StdError::generic_err(MINTER_ERROR)),
    };

    // Store token info
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply,
        mint,
    };

    token_info(&mut deps.storage).save(&data)?;

    if let Some(hook) = msg.init_hook {
        Ok(InitResponse {
            messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: hook.contract_addr,
                msg: hook.msg,
                send: vec![],
            })],
            log: vec![],
        })
    } else {
        Ok(InitResponse::default())
    }
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> HandleResult {
    cw20_handle(deps, env, msg)
}

pub fn migrate<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: MigrateMsg,
) -> MigrateResult {
    cw20_migrate(deps, env, msg)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    cw20_query(deps, msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Uint128, testing::{mock_dependencies, mock_env}};
    use cosmwasm_std::{coins, from_binary, HumanAddr, StdError};
    use cw20::MinterResponse;

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let proxy = HumanAddr::from("master_address");
        let env = mock_env(proxy.clone(), &coins(1000, "earth"));

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
}
