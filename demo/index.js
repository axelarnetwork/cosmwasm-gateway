import { LCDClient, MsgSend, MnemonicKey, MsgInstantiateContract, MsgStoreCode,  getCodeId, getContractAddress, isTxError, StdFee } from '@terra-money/terra.js';

import { Validator } from 'jsonschema';
import { contractNames, load_schemas, load_wasm_binaries, write_contract_infos, read_contract_infos } from './contracts.js';
import { networks, connect, mnemonicKey } from './client.js';
import chalk from 'chalk';
import parseArgs from 'minimist';

const validator = new Validator();
const validate_schema = (...args) => validator.validate(...args);

const Info = chalk.blueBright;
const Success = chalk.greenBright;
const Err = chalk.redBright;

const mustInstantiate = (r) => {
  if(isTxError(r)) {
    console.log(`TX hash: ${Err(r.txhash)}`);
    throw new Error(
      `instantiate failed. code: ${r.code}, codespace: ${r.codespace}, raw_log: ${r.raw_log}`
    );
  }
  console.log(`TX hash: ${Success(r.txhash)}`)
  return r;
}

const parseCliArgs =() => parseArgs(process.argv.slice(2), {
  string: ['networkId'],
  boolean: ['store'],
  default: { store: true, networkId: 'local' },
});

function API(client, wallet) {
  async function store_contracts(binariesMap) {
    console.log("Storing contracts:", Object.keys(binariesMap));
    const contractInfos = Object.create(null);
    for(const name in binariesMap) {
      const msg = new MsgStoreCode(wallet.key.accAddress, binariesMap[name]);
      await wallet
        .createAndSignTx({
          msgs: [msg],
          memo: `storing ${name} contract`,
          // fee: new StdFee(1000000, { uluna: 1000000 } ),
        })
        .then(tx => client.tx.broadcast(tx))
        .then(txResult => {
          if (isTxError(txResult)) {
            console.log(`TX hash: ${Err(txResult.txhash)}`);
            throw new Error(
              `store code failed. code: ${txResult.code}, codespace: ${txResult.codespace}, raw_log: ${txResult.raw_log}`
            );
          }
          console.log(`TX hash: ${Success(txResult.txhash)}`)
          contractInfos[name] = {
            storeResult: txResult,
            codeId: getCodeId(txResult)
          };
        })
        .catch(err => {
          console.log(`Failed to store ${Info(name)} contract`, Err(err));
          throw err;
        });
    }
    return contractInfos;
  }

  async function instantiate_contract(codeId, initMsg, initSchema) {
    if(initSchema) {
      const vres = validate_schema(initMsg, initSchema);
      if(vres.errors.length > 0) {
        return new Error(vres.errors)
      }
    }
    const msg = new MsgInstantiateContract(
      wallet.key.accAddress,
      parseInt(codeId),
      initMsg,
      {}, // init coins
      false // migratable
    );
    console.log(msg);

    let tx;
    try {
      tx = await wallet.createAndSignTx({
        msgs: [msg],
      });
    } catch({response: { data }}) {
      console.log(Err(`Failed to instantiate contract using code_id ${codeId}`));
      data && console.log(data);
      return;
    }

    const txRes = mustInstantiate(await client.tx.broadcast(tx));

    const contractAddress = getContractAddress(txRes);
    return contractAddress;
  }

  return {
    instantiate_contract,
    store_contracts,
  }
}

async function run() {
  const argv = parseCliArgs();
  console.log({argv: argv})
  const { networkId, store } = argv;

  const client = connect(networks[networkId])
  console.log(`Connected terra client to ${Info(networkId)} network`);
  const wallet = client.wallet(mnemonicKey);
  console.log(`Using account ${Info(wallet.key.accAddress)} as sender`);
  const api = API(client, wallet);

  const schemas = load_schemas(contractNames);
  const binaries = load_wasm_binaries(contractNames);

  let contractInfos;
  if(store) {
    contractInfos = await api.store_contracts(binaries);
    write_contract_infos(contractInfos);
  } else {
    contractInfos = read_contract_infos();
    console.log('using contracts:', contractInfos);
  }

  const init_contract = (name, initMsg) => api.instantiate_contract(contractInfos[name].codeId, initMsg, schemas[name].init_msg);

  const crypto_addr = await init_contract('axelar_crypto', {});
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