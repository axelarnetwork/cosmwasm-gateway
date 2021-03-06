import { MsgExecuteContract } from "@terra-money/terra.js";

export function initMsgToWasmMsg(initMsg, label = "") {
  const {
    owner,
    code_id,
    init_msg,
    init_coins,
    migratable,
  } = initMsg.toData().value;

  return {
    wasm: {
      instantiate: {
        code_id: +code_id,
        msg: init_msg,
        send: init_coins,
        label: label,
      },
    },
  };
}

export function executeMsgToWasmMsg(execMsg) {
  const { sender, contract, execute_msg, coins } = execMsg.toData().value;

  return {
    wasm: {
      execute: {
        msg: execute_msg,
        contract_addr: contract,
        send: coins,
      },
    },
  };
}

export const WasmExecuteMsg = (contract_addr, msg, coins) =>
  executeMsgToWasmMsg(new MsgExecuteContract("", contract_addr, msg, coins));

export function RawWasmInitMsg(code_id, msg, send = {}, label = "") {
  return {
    wasm: {
      instantiate: {
        code_id,
        msg,
        send,
        label,
      },
    },
  };
}

export function RawWasmExecuteMsg(contract_addr, msg, send = {}) {
  return {
    wasm: {
      execute: {
        contract_addr,
        msg,
        send,
      },
    },
  };
}
