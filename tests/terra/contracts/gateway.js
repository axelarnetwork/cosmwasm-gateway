import {
  AXELAR_TOKEN_FACTORY,
  AXELAR_TOKEN,
  AXELAR_GATEWAY,
  AXELAR_CRYPTO,
} from "../contracts.js";
import { dictToB64 } from "@terra-money/terra.js";
import { keccak256 } from "ethereum-cryptography/keccak.js";

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
    handleSchema
  );

export const gatewayExecuteSignedFn = (
  client,
  wallet,
  contractApi,
  gatewayAddress,
  handleSchema
) => async (wasmMsgs, registerNames = []) => {

  let res = await client.wasm.contractQuery(gatewayAddress, { config: {} });
  console.log(res);

  const serialized = serializeWasmMessages(wasmMsgs);
  const nonce = Buffer.alloc(8);
  nonce.writeBigUInt64BE(BigInt(res.nonce));

  const digest = keccak256(Buffer.concat([serialized, nonce]));
  const sig = await wallet.key.sign(digest);

  return contractApi.execute_contract(
    gatewayAddress,
    {
      execute_signed: {
        msgs: [...wasmMsgs],
        register: [...registerNames],
        sig: sig.toJSON().data,
      },
    },
    handleSchema
  );
};

function serializeWasmMessages(wasmMsgs) {
  return Buffer.concat(wasmMsgs.map(m => Buffer.from(dictToB64(m), 'base64')));
}
