import { LCDClient, MsgSend, MnemonicKey, MsgInstantiateContract, MsgStoreCode } from '@terra-money/terra.js';
import secp256k1 from 'secp256k1';

export const networks = {
  // soju testnet
  soju:{
    URL: 'https://soju-lcd.terra.dev',
    chainID: 'soju-0014',
  },
  // LocalTerra
  local: {
    URL: 'http://localhost:1317',
    chainID: 'localterra',
    gasPrices: '0.15uluna'
  },
};

export const mnemonicKey = new MnemonicKey({
  mnemonic: 'satisfy adjust timber high purchase tuition stool faith fine install that you unaware feed domain license impose boss human eager hat rent enjoy dawn',
  // 'notice oak worry limit wrap speak medal online prefer cluster roof addict wrist behave treat actual wasp year salad speed social layer crew genius',
});

export const connect = (network) => new LCDClient(network);

export function pubKeyFromPrivKey(privateKey) {
    const publicKey = secp256k1.publicKeyCreate(
      new Uint8Array(privateKey),
      true
    );
    return Buffer.from(publicKey);
}
