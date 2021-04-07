import { gatewayExecuteFn } from './contracts/gateway.js';
import { WasmExecuteMsg, executeMsgToWasmMsg } from './wasm.js';
import { AXELAR_GATEWAY } from './contracts.js';

export default function TransferApi(wallet, client, contractApi, gatewayAddress, tokenParams, tokenAddress) {
  const executeAsGateway = gatewayExecuteFn(contractApi, gatewayAddress);

  async function mint(recipient, amount) {
    const msg = WasmExecuteMsg(tokenAddress, { mint: { recipient, amount }});
    console.dir({ mintMsg: msg }, {depth: 10})
    await executeAsGateway([msg]);
    console.log(`Minted ${amount} ${tokenParams.symbol} to ${recipient}`)
  }

  async function withdraw(crossChainAddr, amount) {
    await contractApi.execute_contract(tokenAddress, { withdraw: { recipient: crossChainAddr, amount }});
    console.log(`Withdrew ${amount} ${tokenParams.symbol} to ${crossChainAddr}`)
  }

  async function burn(amount) {
    const msg = WasmExecuteMsg(tokenAddress, { burn: { amount }});
    console.dir({ mintMsg: msg }, {depth: 10})
    await executeAsGateway([msg]);
    console.log(`Burned ${amount} ${tokenParams.symbol}`)
  }

  return {
    mint,
    withdraw,
    burn,
  }
}
