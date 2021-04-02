import * as fs from 'fs';

const wasmPath = '../artifacts/'

export const contractNames = ['crypto_verify', 'gateway', 'axelar_token', 'token_factory']

export const load_wasm_binaries = (names) => names.reduce((bins, name) => {
  bins[name] = fs.readFileSync(wasmPath + name + '.wasm').toString('base64');
  return bins;
} , Object.create(null));

