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
import { executeMsgToWasmMsg, initMsgToWasmMsg } from "./wasm.js";
import { gatewayExecuteFn } from "./contracts/gateway.js";
import TransferApi from "./transfer.js";

import { networks, connect, mnemonicKey } from "./client.js";
import chalk from "chalk";
import parseArgs from "minimist";
import assert from "assert";

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
    boolean: ["store", "redeploy"],
    default: {
      store: true,
      redeploy: false,
      networkId: "local",
      gateway_addr: "",
      factory_addr: "",
    },
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
    console.dir(msg, { depth: 10 });

    let tx;
    try {
      tx = await wallet.createAndSignTx({ msgs: [msg] });
    } catch ({ response: { data } }) {
      console.log(Err(`Failed to execute contract at ${contractAddr}`));
      data && console.log(data);
      throw new Error(data);
    }

    return txMustSucceed(await client.tx.broadcast(tx), "execute");
  }

  return {
    execute_contract,
    instantiate_contract,
    store_contracts,
  };
}

async function run() {
  const argv = parseCliArgs();
  let { networkId, store, redeploy, gateway_addr, factory_addr } = argv;

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

  // Merge schemas
  Object.keys(schemas).forEach((name) => {
    contractInfos[name].schemas = schemas[name];
  });

  // Extract loaded addresses if we want to use existing contracts
  let addresses = redeploy
    ? {}
    : Object.keys(contractInfos).reduce((a, n) => ({ ...a, [n]: contractInfos[n].address }), {});
  if (gateway_addr?.length > 0) addresses[AXELAR_GATEWAY] = gateway_addr;
  if (factory_addr?.length > 0) addresses[AXELAR_TOKEN_FACTORY] = factory_addr;

  const tokenParams = {
    name: "Satoshi",
    symbol: "satoshi",
    decimals: 8,
    cap: "1000000",
  };

  addresses = await deployAxelarTransferContracts(
    wallet,
    client,
    contractApi,
    contractInfos,
    tokenParams,
    addresses
  );

  // Merge addresses
  Object.keys(addresses).forEach((name) => {
    contractInfos[name].address = addresses[name];
  });
  write_contract_infos(contractInfos);

  const transfer = TransferApi(
    wallet,
    client,
    contractApi,
    addresses[AXELAR_GATEWAY],
    tokenParams,
    addresses[AXELAR_TOKEN]
  );

  // Assert gateway is the authorized token minter
  let res = await client.wasm.contractQuery(addresses[AXELAR_TOKEN], {
    minter: {},
  });
  assert(res.minter == addresses[AXELAR_GATEWAY]);

  const btcAddr = 'tb1qw99lg2um87u0gxx4c8k9f9h8ka0tcjcmjk92np';

  // Mint, withdraw, then consolidate
  await transfer.mint(wallet.key.accAddress, "100");
  await transfer.withdraw(btcAddr, "100");
  await transfer.burn("100");
}

async function deployAxelarTransferContracts(
  wallet,
  client,
  contractApi,
  contractInfos,
  tokenParams,
  addresses = {}
) {
  const init_contract = (name, initMsg) =>
    contractApi.instantiate_contract(
      contractInfos[name].codeId,
      initMsg
      //contractInfos[name].schemas.init_msg
    );

  const logDeployed = (name, address) =>
    console.log(
      `\n+++++ Deployed ${Info(name)} contract at ${Info(address)}\n`
    );

  if (!addresses[AXELAR_GATEWAY]) {
    addresses[AXELAR_CRYPTO] = await init_contract(AXELAR_CRYPTO, {});
    logDeployed(AXELAR_CRYPTO, addresses[AXELAR_CRYPTO]);

    addresses[AXELAR_GATEWAY] = await init_contract(AXELAR_GATEWAY, {
      owner: wallet.key.accAddress,
      // public_key: wallet.key.rawPubKey.toString('base64'),
      public_key: COMPRESSED_BASE64_PUB_KEY,
      crypto_contract_addr: addresses[AXELAR_CRYPTO],
    });
    logDeployed(AXELAR_GATEWAY, addresses[AXELAR_GATEWAY]);
  }

  const executeAsGateway = gatewayExecuteFn(
    contractApi,
    addresses[AXELAR_GATEWAY]
  );

  // Deploy and register the token factory
  if (!addresses[AXELAR_TOKEN_FACTORY]) {
    const registerName = AXELAR_TOKEN_FACTORY;
    console.log(`Deploying token factory, registered as '${registerName}'`);
    const wasmMsg = initMsgToWasmMsg(
      new MsgInstantiateContract(
        addresses[AXELAR_GATEWAY],
        parseInt(contractInfos[AXELAR_TOKEN_FACTORY].codeId),
        {
          owner: addresses[AXELAR_GATEWAY],
          token_code_id: parseInt(contractInfos[AXELAR_TOKEN].codeId),
          init_hook: {
            contract_addr: addresses[AXELAR_GATEWAY],
            msg: dictToB64({ register: { name: registerName } }),
          },
        },
        {}, // init coins
        false // migratable
      )
    );
    console.dir(wasmMsg, { depth: 10 });

    await executeAsGateway([wasmMsg], [registerName]);

    addresses[AXELAR_TOKEN_FACTORY] = (
      await client.wasm.contractQuery(
        addresses[AXELAR_GATEWAY],
        { contract_address: { name: registerName } } // query msg
      )
    ).contract_addr;
    logDeployed(AXELAR_TOKEN_FACTORY, addresses[AXELAR_TOKEN_FACTORY]);
  }

  if (!addresses[AXELAR_TOKEN]) {
    // Deploy a CW20 token
    const deployTokenMsg = executeMsgToWasmMsg(
      new MsgExecuteContract("", addresses[AXELAR_TOKEN_FACTORY], {
        deploy_token: tokenParams,
      })
    );

    console.dir({ deployTokenMsg });

    await executeAsGateway([deployTokenMsg]);

    // Retrieve token contract address from token factory
    addresses[AXELAR_TOKEN] = (
      await client.wasm.contractQuery(addresses[AXELAR_TOKEN_FACTORY], {
        token_address: { symbol: tokenParams.symbol },
      })
    ).token_addr;
    logDeployed(AXELAR_TOKEN, addresses[AXELAR_TOKEN]);
  }

  return addresses;
}

async function assertCanInitToken(init_contract, wallet) {
  const token_addr = await init_contract(AXELAR_TOKEN, {
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
  const token_factory_addr = await init_contract(AXELAR_TOKEN_FACTORY, {
    owner: wallet.key.accAddress,
    token_code_id: parseInt(contractInfos[AXELAR_TOKEN].codeId),
  });
  return true;
}

run().catch(console.log);
