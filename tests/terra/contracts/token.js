import { gatewayExecuteFn } from './gateway.js';
import { WasmExecuteMsg, executeMsgToWasmMsg } from '../wasm.js';
import { AXELAR_GATEWAY } from '../contracts.js';
import { logMsg } from '../utils.js';

export default function TransferApi(wallet, client, contractApi) {
  return (executeAsGateway, tokenParams, tokenAddress) => {
    async function mint(recipient, amount) {
      const msg = WasmExecuteMsg(tokenAddress, { mint: { recipient, amount }});
      logMsg({ mintMsg: msg })
      await executeAsGateway([msg]);
      console.log(`\nMinted ${amount} ${tokenParams.symbol} to ${recipient}\n`)
    }

    async function withdraw(crossChainAddr, amount) {
      await contractApi.execute_contract(tokenAddress, { withdraw: { recipient: crossChainAddr, amount }});
      console.log(`\nWithdrew ${amount} ${tokenParams.symbol} to ${crossChainAddr}\n`)
    }

    async function burn(amount) {
      const msg = WasmExecuteMsg(tokenAddress, { burn: { amount }});
      logMsg({ burnMsg: msg })
      await executeAsGateway([msg]);
      console.log(`\nBurned ${amount} ${tokenParams.symbol}\n`)
    }

    return {
      mint,
      withdraw,
      burn,
    };
  }
}
