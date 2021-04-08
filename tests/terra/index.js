import parseArgv from "minimist";
import assert from "assert";
import { Validator } from "jsonschema";
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
import {
  contractNames,
  load_schemas,
  load_wasm_binaries,
  write_deployments,
  read_deployments,
  AXELAR_TOKEN_FACTORY,
  AXELAR_TOKEN,
  AXELAR_GATEWAY,
  AXELAR_CRYPTO,
} from "./contracts.js";
import { networks, connect, pubKeyFromPrivKey } from "./client.js";
import { setVerbose, logMsg, Info, Success, Err } from "./utils.js";
import { WasmExecuteMsg, WasmInstantiateMsg } from "./wasm.js";
import { gatewayExecuteFn, gatewayExecuteSignedFn } from "./contracts/gateway.js";
import TokenApi from "./contracts/token.js";

const validator = new Validator();
const validate_schema = (...args) => validator.validate(...args);

const BASE64_COMPRESSED_PUB_KEY =
  "An4JQUJX6KTbh6CvqmDLPhe6knWdqfKYjDvkCl2QE1oc";

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
  parseArgv(process.argv.slice(2), {
    string: ["networkId", "gateway_addr", "factory_addr"],
    boolean: ["store", "redeploy", "verbose", "metatx"],
    default: {
      store: true,
      redeploy: true,
      metatx: false,
      verbose: false,
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
    logMsg(msg);

    let tx;
    try {
      tx = await wallet.createAndSignTx({ msgs: [msg] });
    } catch ({ response: { data } }) {
      console.log(
        Err(`Failed to instantiate contract using code_id ${codeId}`)
      );
      data && console.log(data)
      throw new Error(data);
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
    logMsg(msg);

    let tx;
    try {
      tx = await wallet.createAndSignTx({ msgs: [msg] });
    } catch ({ response: { data } }) {
      console.log(Err(`Failed to execute contract at ${contractAddr}`));
      data && console.log(data)
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
  let { networkId, store, redeploy, gateway_addr, factory_addr, verbose, metatx } = argv;
  setVerbose(verbose);

  const network = networks[networkId];
  const client = connect(network);
  console.log(`Connected terra client to ${Info(networkId)} network`);

  const mnemonicKey = new MnemonicKey({ mnemonic: network.mnemonic });
  const wallet = client.wallet(mnemonicKey);

  console.log(`Using account ${Info(wallet.key.accAddress)} as sender`);
  const contractApi = ContractApi(client, wallet);

  const schemas = load_schemas(contractNames);
  const binaries = load_wasm_binaries(contractNames);

  // Load existing deployment info
  let deployments = read_deployments();
  if (store) {
    deployments[networkId] = await contractApi.store_contracts(binaries);
    write_deployments(deployments);
  } else {
    console.log("Using contracts:", deployments[networkId]);
  }
  let contractsInfo = deployments[networkId];

  // Extract loaded addresses if we want to use existing contracts
  let addresses = redeploy
    ? {}
    : Object.keys(contractsInfo).reduce((a, n) => ({ ...a, [n]: contractsInfo[n].address }), {});
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
    contractsInfo,
    tokenParams,
    addresses,
    { metatx }
  );

  // Merge addresses
  Object.keys(addresses).forEach((name) => {
    contractsInfo[name].address = addresses[name];
  });
  write_deployments(deployments);

  const executeAsGateway = metatx
    ? gatewayExecuteSignedFn(
      client,
      wallet,
      contractApi,
      addresses[AXELAR_GATEWAY]
    )
    : gatewayExecuteFn(
      contractApi,
      addresses[AXELAR_GATEWAY]
    );

  const transfer = TokenApi( wallet, client, contractApi)(
    executeAsGateway,
    tokenParams,
    addresses[AXELAR_TOKEN]
  );

  // Assert gateway is the authorized token minter
  let res = await client.wasm.contractQuery(addresses[AXELAR_TOKEN], { minter: {} });
  assert(res.minter == addresses[AXELAR_GATEWAY]);

  const btcAddr = "tb1qw99lg2um87u0gxx4c8k9f9h8ka0tcjcmjk92np";

  // Mint, withdraw, then consolidate
  await transfer.mint(wallet.key.accAddress, "100");
  await transfer.withdraw(btcAddr, "100");
  await transfer.burn("100");
}

async function deployAxelarTransferContracts(
  wallet,
  client,
  contractApi,
  contractsInfo,
  tokenParams,
  addresses = {},
  opts = {},
) {
  const { metatx } = opts;

  const init_contract = (name, initMsg) =>
    contractApi.instantiate_contract(
      contractsInfo[name].codeId,
      initMsg
      //contractsInfo[name].schemas.init_msg
    );

  const logDeployed = (name, address) =>
    console.log(`\nDeployed ${Info(name)} contract at ${Info(address)}\n`);

  if (!addresses[AXELAR_GATEWAY]) {
    addresses[AXELAR_CRYPTO] = await init_contract(AXELAR_CRYPTO, {});
    logDeployed(AXELAR_CRYPTO, addresses[AXELAR_CRYPTO]);

    addresses[AXELAR_GATEWAY] = await init_contract(AXELAR_GATEWAY, {
      owner: wallet.key.accAddress,
      public_key: pubKeyFromPrivKey(wallet.key.privateKey).toString("base64"),
      crypto_contract_addr: addresses[AXELAR_CRYPTO],
    });
    logDeployed(AXELAR_GATEWAY, addresses[AXELAR_GATEWAY]);
  }

  const executeAsGateway = metatx
    ? gatewayExecuteSignedFn(
      client,
      wallet,
      contractApi,
      addresses[AXELAR_GATEWAY]
    )
    : gatewayExecuteFn(
      contractApi,
      addresses[AXELAR_GATEWAY]
    );

  // Deploy and register the token factory
  if (!addresses[AXELAR_TOKEN_FACTORY]) {
    const registerName = AXELAR_TOKEN_FACTORY;
    console.log(`Deploying token factory, registered as '${registerName}'`);

    const wasmMsg = WasmInstantiateMsg(
        parseInt(contractsInfo[AXELAR_TOKEN_FACTORY].codeId),
        {
          owner: addresses[AXELAR_GATEWAY],
          token_code_id: parseInt(contractsInfo[AXELAR_TOKEN].codeId),
          init_hook: {
            contract_addr: addresses[AXELAR_GATEWAY],
            msg: dictToB64({ register: { name: registerName } }),
          },
        },
      );
    logMsg(wasmMsg);

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
    const deployTokenMsg = WasmExecuteMsg(addresses[AXELAR_TOKEN_FACTORY], { deploy_token: tokenParams });
    logMsg(deployTokenMsg);

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
