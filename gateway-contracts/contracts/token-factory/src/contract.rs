use cosmwasm_std::{
    log, to_binary, Api, Binary, Env, Extern, HandleResponse, InitResponse, Querier, StdError,
    StdResult, Storage,
};

use crate::msg::{HandleMsg, InitMsg, OwnerResponse, QueryMsg};
use crate::state::{owner, owner_read};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    owner(&mut deps.storage).save(&deps.api.canonical_address(&env.message.sender)?)?;

    Ok(InitResponse {
        log: vec![
            log("ownership_transferred:previous_owner", "0"),
            log("ownership_transferred:new_owner", env.message.sender),
        ],
        messages: vec![]
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

pub fn only_owner<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
) -> StdResult<()> {
    if deps.api.canonical_address(&env.message.sender)? != owner_read(&deps.storage).load()? {
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
    only_owner(&deps, &env)?;

    Ok(HandleResponse::default())
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner {} => to_binary(&query_owner(deps)?),
    }
}

fn query_owner<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<OwnerResponse> {
    let owner = owner_read(&deps.storage).load()?;
    Ok(OwnerResponse { owner: owner })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, from_binary, HumanAddr, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let master_address = HumanAddr::from("master_address");
        let canon_master_address = deps.api.canonical_address(&master_address).unwrap();

        let env = mock_env(master_address, &coins(1000, "earth"));

        let res = init(&mut deps, env, InitMsg {}).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(&deps, QueryMsg::GetOwner {}).unwrap();
        let value: OwnerResponse = from_binary(&res).unwrap();
        assert_eq!(canon_master_address, value.owner);
    }

    #[test]
    fn authorization() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let master_address = HumanAddr::from("master_address");
        let env = mock_env(master_address.clone(), &coins(2, "token"));
        let _res = init(&mut deps, env, InitMsg {}).unwrap();

        let unauth_env = mock_env("anyone", &coins(2, "token"));

        let res = only_owner(&deps, &unauth_env.clone());
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }

        let env = mock_env(master_address.clone(), &coins(2, "token"));
        let res = only_owner(&deps, &env.clone());
        match res {
            Err(StdError::Unauthorized { .. }) => panic!("Owner should be authorized"),
            _ => {}
        }
    }
}
