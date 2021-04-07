import { LCDClient, MsgSend, MnemonicKey, MsgInstantiateContract, MsgStoreCode } from '@terra-money/terra.js';
import secp256k1 from 'secp256k1';

export const networks = {
  // tequila testnet
  tequila:{
    URL: 'https://tequila-lcd.terra.dev',
    chainID: 'tequila-0004',
    mnemonic: 'dress chimney never comic novel teach fun vintage ski bird estate promote category record case seven unfold web during wagon notable hold naive scout',
  },

  // LocalTerra
  local: {
    URL: 'http://localhost:1317',
    chainID: 'localterra',
    gasPrices: '0.15uluna',
    mnemonic: 'satisfy adjust timber high purchase tuition stool faith fine install that you unaware feed domain license impose boss human eager hat rent enjoy dawn',
  },
};

export const connect = (network) => new LCDClient(network);

export function pubKeyFromPrivKey(privateKey) {
    const publicKey = secp256k1.publicKeyCreate(
      new Uint8Array(privateKey),
      true
    );
    return Buffer.from(publicKey);
}
