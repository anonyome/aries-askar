import React from 'react';
import {Button, SafeAreaView} from 'react-native';
import {Key, KeyAlgs, registerAriesAskar, Store} from 'aries-askar-shared';
import {ReactNativeAriesAskar} from 'aries-askar-react-native';
import {
  setupStore,
  storeInsert,
  storeKeyStore,
  storeProfile,
  storeRemove,
  storeRemoveAll,
  storeReplace,
  storeScan,
  storeTransactionBasic,
} from './tests/store.test';
import {keyBlsG1Keygen, keyBlsG2Keygen, keyEd25519} from './tests/keys.test';
import {cryptoBoxSeal} from './tests/cryptoBox.test';
import {
  joseEcdh1puDirect,
  joseEcdh1puWrappedExpected,
  joseEcdhEsDirect,
  joseEcdhEsWrapped,
} from './tests/joseEcdh.test';

const doAsyncTest = async (
  cb: (store: Store) => Promise<number | undefined>,
  name: string,
) => {
  try {
    const store = await setupStore();
    const res = await cb(store);
    if (res) {
      console.error(`Test ${name}:${res} failed`);
    } else {
      console.log(`Test ${name} succeeded`);
    }
    store.close(true);
  } catch (e) {
    console.error(`Test ${name} failed with mesage: ${e}`);
  }
};

const doTest = (cb: () => number | undefined, name: string) => {
  try {
    const res = cb();
    if (res) {
      console.error(`Test ${name}:${res} failed`);
    } else {
      console.log(`Test ${name} succeeded`);
    }
  } catch (e) {
    console.error(`Test ${name} failed with mesage: ${e}`);
  }
};

export const App = () => {
  registerAriesAskar({askar: new ReactNativeAriesAskar()});
  const storeTestCases: Record<
    string,
    (store: Store) => Promise<number | undefined>
  > = {
    'Store: insert': storeInsert,
    'Store: replace': storeReplace,
    'Store: remove': storeRemove,
    'Store: remove all': storeRemoveAll,
    'Store: Scan': storeScan,
    'Store: Transaction Basic': storeTransactionBasic,
    'Store: profile': storeProfile,
    'Store: key store': storeKeyStore,
  };

  const keyTestCases: Record<string, () => any> = {
    'Key: BLS G2 Keygen': keyBlsG2Keygen,
    'Key: BLS G1 Keygen': keyBlsG1Keygen,
    'Key: Ed25519': keyEd25519,
  };

  const cryptoBoxTestCases: Record<string, () => any> = {
    'CryptoBox: seal': cryptoBoxSeal,
  };

  const joseEcdhTestCases: Record<string, () => any> = {
    'Jose ECDH: es direct': joseEcdhEsDirect,
    'Jose ECDH: es wrapped': joseEcdhEsWrapped,
    'Jose ECDH: 1pu direct': joseEcdh1puDirect,
    'Jose ECDH: 1pu wrapped expected': joseEcdh1puWrappedExpected,
  };

  return (
    <SafeAreaView>
      <Button
        title="dbg"
        onPress={() => {
          console.log(Key.generate(KeyAlgs.Ed25519, 0));
        }}
      />
      <Button
        title="Store: All"
        onPress={() => {
          Object.entries(storeTestCases).map(
            async ([funcName, cb]) => await doAsyncTest(cb, funcName),
          );
        }}
      />
      <Button
        title="Key: All"
        onPress={() => {
          Object.entries(keyTestCases).map(async ([funcName, cb]) =>
            doTest(cb, funcName),
          );
        }}
      />
      <Button
        title="CryptoBox: All"
        onPress={() => {
          Object.entries(cryptoBoxTestCases).map(async ([funcName, cb]) =>
            doTest(cb, funcName),
          );
        }}
      />
      <Button
        title="Jose ECDH: All"
        onPress={() => {
          Object.entries(joseEcdhTestCases).map(async ([funcName, cb]) =>
            doTest(cb, funcName),
          );
        }}
      />
    </SafeAreaView>
  );
};
