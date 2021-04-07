import { AXELAR_TOKEN_FACTORY, AXELAR_TOKEN, AXELAR_GATEWAY, AXELAR_CRYPTO } from "../contracts.js"

// Execute a vector of WASM messages using the gateway as a proxy.
// Each message will be executed with the gateway as the sender.
export const gatewayExecuteFn = (contractApi, gatewayAddress, handleSchema) => (
  wasmMsgs,
  registerNames = []
) =>
  contractApi.execute_contract(
    gatewayAddress,
    {
      execute: {
        msgs: [...wasmMsgs],
        register: [...registerNames],
      },
    },
    handleSchema,
  );
