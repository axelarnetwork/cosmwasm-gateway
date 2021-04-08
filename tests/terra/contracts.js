import * as fs from "fs";

const wasmDir = "../../artifacts/";
const contractsDir = "../../contracts/";

export const AXELAR_CRYPTO = "axelar_crypto";
export const AXELAR_GATEWAY = "axelar_gateway";
export const AXELAR_TOKEN = "axelar_token";
export const AXELAR_TOKEN_FACTORY = "axelar_token_factory";

export const contractNames = [ AXELAR_CRYPTO, AXELAR_GATEWAY, AXELAR_TOKEN, AXELAR_TOKEN_FACTORY ];

export const load_wasm_binaries = (names) =>
  names.reduce((bins, name) => {
    bins[name] = fs.readFileSync(wasmDir + name + ".wasm").toString("base64");
    return bins;
  }, Object.create(null));

const toDirName = (name) => name.replace("axelar_", "").replace("_", "-");

export const load_schemas = (names) =>
  names.reduce((contracts, name) => {
    const schemaDir = `${contractsDir}${toDirName(name)}/schema`;
    const fNames = fs.readdirSync(schemaDir);

    contracts[name] = fNames.reduce((schemas, name) => {
      schemas[name.replace(".json", "")] = JSON.parse(
        fs.readFileSync(`${schemaDir}/${name}`).toString()
      );
      return schemas;
    }, Object.create(null));

    return contracts;
  }, Object.create(null));

const deploymentsPath = "./deployments.json";

// Merge schemas
export const merge_schemas = (contractInfo, schemas) =>
  Object.keys(schemas).forEach((name) => {
    contractInfos[name].schemas = schemas[name];
  });

// deployments: { [network]: { [contract]: {} } }
export const write_deployments = (infos, path = deploymentsPath) => {
  fs.writeFileSync(path, JSON.stringify(infos, null, 2));
};

export const read_deployments = (path = deploymentsPath) => {
  return fs.existsSync(path) ? JSON.parse(fs.readFileSync(path).toString()) : {};
};
