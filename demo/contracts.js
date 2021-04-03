import * as fs from 'fs';

const wasmDir = '../artifacts/'
const contractsDir = '../contracts/'

export const contractNames = ['axelar_crypto', 'axelar_gateway', 'axelar_token', 'axelar_token_factory']

export const load_wasm_binaries = (names) => names.reduce((bins, name) => {
  bins[name] = fs.readFileSync(wasmDir + name + '.wasm').toString('base64');
  return bins;
} , Object.create(null));

let s = "string";
const toDirName = name => name.replace("axelar_", "").replace("_","-");

export const load_schemas = (names) => names.reduce((contracts, name) => {
  const schemaDir = `${contractsDir}${toDirName(name)}/schema`;
  const fNames = fs.readdirSync(schemaDir);

  contracts[name] = fNames.reduce((schemas, name) => {
    schemas[name.replace('.json', '')] = JSON.parse(fs.readFileSync(`${schemaDir}/${name}`).toString());
    return schemas;
  }, Object.create(null));

  return contracts;
} , Object.create(null));

const infosPath = './contract_info.json';

export const write_contract_infos = (contractInfos) => {
  const out = Object.keys(contractInfos).reduce((out, name) => {
    const { codeId } = contractInfos[name]; // select properties we wish to write
    out[name] = { codeId };
    return out;
  }, Object.create(null));

  fs.writeFileSync(infosPath, JSON.stringify(out));
}

export const read_contract_infos = () => {
  return JSON.parse(fs.readFileSync(infosPath).toString());
}
