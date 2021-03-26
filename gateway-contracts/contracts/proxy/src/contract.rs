use std::fmt;

use cosmwasm_std::{Api, Binary, CosmosMsg, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier, StdError, StdResult, Storage, log, to_binary};
use schemars::JsonSchema;

use crate::state::{config, config_read, State};
use cw1::{};
use cw1_whitelist::{contract::{handle_execute, handle_freeze, handle_update_admins, init, query_admin_list, query_can_send}, state::{AdminList, admin_list, admin_list_read}};

use axelar_gateway::proxy::{HandleMsg, InitMsg, QueryMsg};

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::ExecuteWrapped { msgs, sig } => handle_execute_wrapped(deps, env, msgs, sig),
        HandleMsg::Execute { msgs } => handle_execute(deps, env, msgs),
        HandleMsg::Freeze {} => handle_freeze(deps, env),
        HandleMsg::UpdateAdmins { admins } => handle_update_admins(deps, env, admins),
    }
}

// execute messages authenticated with the admin's signature
pub fn handle_execute_wrapped<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msgs: Vec<CosmosMsg<T>>,
    sig: String,
) -> StdResult<HandleResponse<T>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{

    // todo: recover signer canon address from hash of packed messages
    let recovered = HumanAddr::from("");

    if !can_send(&deps, &recovered)? {
        Err(StdError::unauthorized())
    } else {
        let mut res = HandleResponse::default();
        res.messages = msgs;
        res.log = vec![log("action", "execute")];
        Ok(res)
    }
}

fn can_send<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    sender: &HumanAddr,
) -> StdResult<bool> {
    let cfg = admin_list_read(&deps.storage).load()?;
    let can = cfg.is_admin(&deps.api.canonical_address(sender)?);
    Ok(can)
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        //QueryMsg::AdminList {} => to_binary(&query_count(deps)?),
        _ => to_binary(""),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{HumanAddr, testing::{mock_dependencies, mock_env}};
    use cosmwasm_std::{coins, from_binary, StdError};
    use cw1_whitelist::msg::AdminListResponse;

    #[test]
    fn initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let axelar = HumanAddr::from("axelar");
        let anyone = HumanAddr::from("anyone");

        // instantiate the contract
        let msg = InitMsg {
                admins: vec![axelar.clone()],
                mutable: true,
        };

        let env = mock_env(axelar.clone(), &[]);
        let res = init(&mut deps, env, msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![axelar.clone()],
            mutable: true,
        };
        assert_eq!(query_admin_list(&deps).unwrap(), expected);

        // we can just call .unwrap() to assert this was a success
        assert_eq!(0, res.messages.len());

    }
}
