use cosmwasm_std::{
    HandleResponse, log, HumanAddr,
    Api, Binary, Uint128, CosmosMsg, Env, Extern, HandleResult, InitResponse, MigrateResult, Querier, StdError, StdResult, Storage, WasmMsg, to_binary
};

use cw2::set_contract_version;
use cw20_base::contract::{
    create_accounts, handle_send as cw20_handle_send, handle_transfer as cw20_handle_transfer, migrate as cw20_migrate, query as cw20_query, query_balance, query_minter, query_token_info, handle_mint as cw20_handle_mint };
use cw20_base::enumerable::{query_all_accounts};
use cw20_base::state::{token_info_read, balances};
use cw20_base::msg::{MigrateMsg};
use cw20_base::state::{token_info, MinterData, TokenInfo};

use axelar_gateway_contracts::token::{InitMsg,HandleMsg,QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const MINTER_ERROR: &str = "Must provide minter data";

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
    match msg {
        HandleMsg::Withdraw { recipient, amount } => handle_withdraw(deps, env, recipient, amount),
        HandleMsg::Transfer { recipient, amount } => cw20_handle_transfer(deps, env, recipient, amount),
        HandleMsg::Burn { amount } => handle_burn(deps, env, amount),
        HandleMsg::Send {
            contract,
            amount,
            msg,
        } => cw20_handle_send(deps, env, contract, amount, msg),
        HandleMsg::Mint { recipient, amount } => cw20_handle_mint(deps, env, recipient, amount),
    }
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
    match msg {
        QueryMsg::Balance { address } => to_binary(&query_balance(deps, address)?),
        QueryMsg::TokenInfo {} => to_binary(&query_token_info(deps)?),
        QueryMsg::Minter {} => to_binary(&query_minter(deps)?),
        QueryMsg::AllAccounts { start_after, limit } => {
            to_binary(&query_all_accounts(deps, start_after, limit)?)
        }
    }
}

pub fn handle_withdraw<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    recipient: HumanAddr,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Invalid zero amount"));
    }

    // send to minter (proxy contract will consolidate later by burning)
    let config = token_info_read(&deps.storage).load()?;
    let rcpt_raw = &config.mint.as_ref().unwrap().minter;
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;

    let mut accounts = balances(&mut deps.storage);
    accounts.update(sender_raw.as_slice(), |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    accounts.update(rcpt_raw.as_slice(), |balance: Option<Uint128>| {
        Ok(balance.unwrap_or_default() + amount)
    })?;

    // log recipient to be read verified by Axelar validators
    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "withdraw"),
            log("from", deps.api.human_address(&sender_raw)?),
            log("to", recipient),
            log("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}

pub fn handle_burn<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    amount: Uint128,
) -> StdResult<HandleResponse> {
    if amount == Uint128::zero() {
        return Err(StdError::generic_err("Invalid zero amount"));
    }

    // only the minter may burn
    let config = token_info_read(&deps.storage).load()?;
    if config.mint.is_none()
        || config.mint.as_ref().unwrap().minter
            != deps.api.canonical_address(&env.message.sender)?
    {
        return Err(StdError::unauthorized());
    }

    let sender_raw = deps.api.canonical_address(&env.message.sender)?;

    // lower balance
    let mut accounts = balances(&mut deps.storage);
    accounts.update(sender_raw.as_slice(), |balance: Option<Uint128>| {
        balance.unwrap_or_default() - amount
    })?;
    // reduce total_supply
    token_info(&mut deps.storage).update(|mut info| {
        info.total_supply = (info.total_supply - amount)?;
        Ok(info)
    })?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "burn"),
            log("from", deps.api.human_address(&sender_raw)?),
            log("amount", amount),
        ],
        data: None,
    };
    Ok(res)
}
