import {
  dictToB64,
  LCDClient,
  MsgSend,
  MnemonicKey,
  MsgInstantiateContract,
  MsgStoreCode,
  getCodeId,
  getContractAddress,
  isTxError,
  StdFee,
  MsgExecuteContract,
} from "@terra-money/terra.js";

import { Validator } from "jsonschema";
import {
  contractNames,
  load_schemas,
  load_wasm_binaries,
  write_contract_infos,
  read_contract_infos,
  AXELAR_TOKEN_FACTORY,
  AXELAR_TOKEN,
  AXELAR_GATEWAY,
  AXELAR_CRYPTO,
} from "./contracts.js";
import { networks, connect, mnemonicKey } from "./client.js";
import chalk from "chalk";
import parseArgs from "minimist";

const validator = new Validator();
const validate_schema = (...args) => validator.validate(...args);

const Info = chalk.blueBright;
const Success = chalk.greenBright;
const Err = chalk.redBright;
const COMPRESSED_BASE64_PUB_KEY =
  "WzMsODQsMTA5LDQwLDEwMiwyMTEsMjI3LDEyMyw0MCwxMjAsNjYsMTk4LDU5LDEwMiwxNDYsMjUwLDQ3LDM5LDE2MiwyNDYsMTQ0LDIyNywyNiwxNjUsNTYsMTg0LDMxLDEyNSw2NCwyOSwxMTgsMTM5LDI0N10=";

const txMustSucceed = (r, kind = "transaction") => {
  if (isTxError(r)) {
    console.log(`TX hash: ${Err(r.txhash)}`);
    throw new Error(
      `${kind} failed. code: ${r.code}, codespace: ${r.codespace}, raw_log: ${r.raw_log}`
    );
  }
  console.log(`TX hash: ${Success(r.txhash)}`);
  return r;
};

const parseCliArgs = () =>
  parseArgs(process.argv.slice(2), {
    string: ["networkId", "gateway_addr", "factory_addr"],
    boolean: ["store"],
    default: { store: true, networkId: "local", gateway_addr: "", factory_addr: "" },
  });

function ContractApi(client, wallet) {
  async function store_contracts(binariesMap) {
    console.log("Storing contracts:", Object.keys(binariesMap));
    const contractInfos = Object.create(null);
    for (const name in binariesMap) {
      const msg = new MsgStoreCode(wallet.key.accAddress, binariesMap[name]);
      await wallet
        .createAndSignTx({
          msgs: [msg],
          memo: `storing ${name} contract`,
          // fee: new StdFee(1000000, { uluna: 1000000 } ),
        })
        .then((tx) => client.tx.broadcast(tx))
        .then((txResult) => {
          if (isTxError(txResult)) {
            console.log(`TX hash: ${Err(txResult.txhash)}`);
            throw new Error(
              `store code failed. code: ${txResult.code}, codespace: ${txResult.codespace}, raw_log: ${txResult.raw_log}`
            );
          }
          console.log(`TX hash: ${Success(txResult.txhash)}`);
          contractInfos[name] = {
            storeResult: txResult,
            codeId: getCodeId(txResult),
          };
        })
        .catch((err) => {
          console.log(`Failed to store ${Info(name)} contract`, Err(err));
          throw err;
        });
    }
    return contractInfos;
  }

  async function instantiate_contract(codeId, initMsg, initSchema) {
    if (initSchema) {
      const vres = validate_schema(initMsg, initSchema, { throwFirst: true });
      if (vres.errors.length > 0) {
        return new Error(vres.errors);
      }
    }
    const msg = new MsgInstantiateContract(
      wallet.key.accAddress,
      +codeId,
      initMsg,
      {}, // init coins
      false // migratable
    );
    console.log(msg);

    let tx;
    try {
      tx = await wallet.createAndSignTx({ msgs: [msg] });
    } catch ({ response: { data } }) {
      console.log(
        Err(`Failed to instantiate contract using code_id ${codeId}`)
      );
      data && console.log(data);
      return;
    }

    const txRes = txMustSucceed(await client.tx.broadcast(tx), "instantiate");

    const contractAddress = getContractAddress(txRes);
    return contractAddress;
  }

  async function execute_contract(contractAddr, handleMsg, handleSchema) {
    if (handleSchema) {
      const vres = validate_schema(handleMsg, handleSchema, {
        throwFirst: true,
      });
      if (vres.errors.length > 0) {
        return new Error(vres.errors);
      }
    }
    const msg = new MsgExecuteContract(
      wallet.key.accAddress,
      contractAddr,
      handleMsg
    );
    console.dir(msg);

    let tx;
    try {
      tx = await wallet.createAndSignTx({ msgs: [msg] });
    } catch ({ response: { data } }) {
      console.log(Err(`Failed to execute contract at ${contractAddr}`));
      data && console.log(data);
      throw new Error(data)
    }

    const txRes = txMustSucceed(await client.tx.broadcast(tx), "execute");

    const contractAddress = getContractAddress(txRes);
    return contractAddress;
  }

  return {
    execute_contract,
    instantiate_contract,
    store_contracts,
  };
}

function initMsgToWasmMsg(initMsg, label = "") {
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

function executeMsgToWasmMsg(execMsg) {
  const {
    sender,
    contract,
    execute_msg,
    coins,
  } = execMsg.toData().value;

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

async function run() {
  const argv = parseCliArgs();
  console.log({ argv: argv });
  let { networkId, store, gateway_addr, factory_addr } = argv;

  const client = connect(networks[networkId]);
  console.log(`Connected terra client to ${Info(networkId)} network`);
  const wallet = client.wallet(mnemonicKey);
  console.log(`Using account ${Info(wallet.key.accAddress)} as sender`);
  const contractApi = ContractApi(client, wallet);

  const schemas = load_schemas(contractNames);
  const binaries = load_wasm_binaries(contractNames);

  let contractInfos;
  if (store) {
    contractInfos = await contractApi.store_contracts(binaries);
    write_contract_infos(contractInfos);
  } else {
    contractInfos = read_contract_infos();
    console.log("Using contracts:", contractInfos);
  }

  Object.keys(schemas).forEach((name) => {
    contractInfos[name].schemas = schemas[name];
  });

  let addresses = {};
  if (gateway_addr?.length > 0) addresses.gateway = gateway_addr;
  if (factory_addr?.length > 0) addresses.tokenFactory = factory_addr;

  addresses = await deployAxelarTransferContracts(wallet, client, contractApi, contractInfos, addresses);
  console.log({ addresses });
}

async function deployAxelarTransferContracts(wallet, client, contractApi, contractInfos, addresses = {}) {
  const init_contract = (name, initMsg) =>
    contractApi.instantiate_contract(
      contractInfos[name].codeId,
      initMsg,
      contractInfos[name].schemas.init_msg
    );

  const logDeployed = (name, address) =>
    console.log(
      `\n+++++ Deployed ${Info(name)} contract at ${Info(address)}\n`
    );

  if (!addresses.gateway) {
    addresses.crypto = await init_contract(AXELAR_CRYPTO, {});
    logDeployed(AXELAR_CRYPTO, addresses.crypto);

    addresses.gateway = await init_contract(AXELAR_GATEWAY, {
      owner: wallet.key.accAddress,
      // public_key: wallet.key.rawPubKey.toString('base64'),
      public_key: COMPRESSED_BASE64_PUB_KEY,
      crypto_contract_addr: addresses.crypto,
    });
    logDeployed(AXELAR_GATEWAY, addresses.gateway);
  }

  // Deploy and register the token factory
  if (!addresses.tokenFactory) {
    const registerName = AXELAR_TOKEN_FACTORY;
    console.log(`Deploying token factory, registered as '${registerName}'`);
    const tokenFactoryInitMsg = new MsgInstantiateContract(
      addresses.gateway,
      parseInt(contractInfos[AXELAR_TOKEN_FACTORY].codeId),
      {
        owner: addresses.gateway,
        token_code_id: parseInt(contractInfos[AXELAR_TOKEN].codeId),
        init_hook: {
          contract_addr: addresses.gateway,
          msg: dictToB64({ register: { name: registerName } }),
        },
      },
      {}, // init coins
      false // migratable
    );
    console.log({ tokenFactoryInitMsg });

    const wasmMsg = initMsgToWasmMsg(tokenFactoryInitMsg);
    console.dir(wasmMsg, { depth: 10 });

    await contractApi.execute_contract(
      addresses.gateway,
      {
        execute: { msgs: [wasmMsg], register: [registerName], },
      },
      contractInfos[AXELAR_GATEWAY].schemas.handle_msg
    );

    let { contract_addr } = await client.wasm.contractQuery(
      addresses.gateway,
      { contract_address: { name: registerName } } // query msg
    );
    addresses.tokenFactory = contract_addr;
    logDeployed(AXELAR_TOKEN_FACTORY, addresses.tokenFactory);
  }


  // Deploy a CW20 token
  const tokenParams = {
        name: 'Satoshi',
        symbol: 'satoshi',
        decimals: 8,
        cap: '1000000',
      };

  const deployTokenMsg = executeMsgToWasmMsg(new MsgExecuteContract(
    '',
    addresses.tokenFactory,
    { deploy_token:  tokenParams }
  ));

  console.dir({deployTokenMsg})

  await contractApi.execute_contract(
    addresses.gateway,
    {
      execute: {
        msgs: [deployTokenMsg],
        register: [],
      },
    },
    contractInfos[AXELAR_GATEWAY].schemas.handle_msg
  );
  addresses.satoshiToken = (await client.wasm.contractQuery(
    addresses.tokenFactory,
    { token_address: { symbol: tokenParams.symbol } }
  )).token_addr;
  logDeployed(AXELAR_TOKEN, addresses.satoshiToken);

  return addresses;
}

async function gatewayProxyExecute(contractApi, wasmMsgs, registerNames) {
}

async function assertCanInitToken(init_contract, wallet) {
  const token_addr = await init_contract("axelar_token", {
    owner: wallet.key.accAddress,
    name: "Satoshi",
    symbol: "satoshi",
    decimals: 8,
    initial_balances: [],
    mint: {
      minter: wallet.key.accAddress,
      cap: "1000000",
    },
    // init_hook: {},
  });
  return true;
}

async function assertCanInitTokenFactory(init_contract, wallet, contractInfos) {
  const token_factory_addr = await init_contract("axelar_token_factory", {
    owner: wallet.key.accAddress,
    token_code_id: parseInt(contractInfos["axelar_token"].codeId),
  });
  return true;
}

run().catch(console.log);

