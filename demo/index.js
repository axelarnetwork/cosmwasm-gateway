import { LCDClient, MsgSend, MnemonicKey, MsgInstantiateContract, MsgStoreCode,  getCodeId, isTxError, StdFee } from '@terra-money/terra.js';

import { Validator } from 'jsonschema';
import { contractNames, load_wasm_binaries } from './contracts.js';
import { networks, connect, mnemonicKey } from './client.js';
import chalk from 'chalk';

/* var v = new Validator();
var instance = 4;
var schema = {"type": "number"};
console.log(v.validate(instance, schema));
*/

function store_contracts(client, wallet, binariesMap) {
  console.log("Storing contracts:", Object.keys(binariesMap));
  const storeTxns = Object.keys(binariesMap).map(name => {
    const msg = new MsgStoreCode(wallet.key.accAddress, binariesMap[name]);
    return wallet
      .createAndSignTx({
        msgs: [msg],
        memo: `storing ${name} contract`,
        // fee: new StdFee(1000000, { uluna: 1000000 } ),
      })
      .then(tx => client.tx.broadcast(tx))
      .then(txResult => {
        console.log(`TX hash: ${txResult.txhash}`);

        if (isTxError(txResult)) {
          throw new Error(
            `store code failed. code: ${txResult.code}, codespace: ${txResult.codespace}, raw_log: ${txResult.raw_log}`
          );
        }
        return [name, txResult];
      })
      .catch(err => console.log(`Failed to store ${Info(name)} contract`, Err(err)));
  });

  return Promise.all(storeTxns);
}

const Info = chalk.blueBright;
const Err = chalk.redBright;

async function run() {
  const networkId = 'local';

  const client = connect(networks[networkId])
  console.log(`Connected terra client to ${Info(networkId)} network`);
  const wallet = client.wallet(mnemonicKey);
  console.log(`Using account ${Info(wallet.key.accAddress)} as sender`);

  const binaries = load_wasm_binaries(contractNames);

  const codes = await store_contracts(client, wallet, binaries);
  console.log(codes);
}

run();

function TxOptions() {
    /* msgs: Msg[];
    fee?: StdFee;
    memo?: string;
    gasPrices?: Coins.Input;
    gasAdjustment?: Numeric.Input;
    feeDenoms?: string[];
    account_number?: number;
    sequence?: number; */
}
