import { VERSION, sendCommandAndAccept, BASE_URL, sendCommandExpectFail, toggleBlindSigningSettings } from "./common";
import { expect } from 'chai';
import { describe, it } from 'mocha';
import Axios from 'axios';
import Pokt from "hw-app-pokt";
import * as ed from '@noble/ed25519';

function testTransactionInternal(path: string, txn0: any, blind: boolean, prompts: any[]) {
  const txn = Buffer.from(JSON.stringify(txn0), "utf-8");
  return async () => {
    await sendCommandAndAccept(async (client : Pokt) => {

      const pk = await client.getPublicKey(path);

      if (blind) {
        await toggleBlindSigningSettings();
      }

      // We don't want the prompts from getPublicKey in our result
      await Axios.delete(BASE_URL + "/events");

      const sig = blind
          ? await client.blindSignTransaction(path, txn)
          : await client.signTransaction(path, txn);

      expect(await ed.verify(sig.signature, txn, pk.publicKey) ? "Signature Valid": "Signature Invalid").to.equal("Signature Valid");
    }, prompts);
  }
}

const testTransaction = (path: string, txn: any, prompts: any[]) =>
    testTransactionInternal(path, txn, false, prompts);

const testBlindTransaction = (path: string, txn: any, prompts: any[]) =>
    testTransactionInternal(path, txn, true, prompts);

// These tests have been extracted interacting with the testnet via the cli.

const exampleSend = {
  "chain_id": "testnet",
  "entropy": "-7780543831205109370",
  "fee": [
    {
      "amount": "12000",
      "denom": "upokt"
    }
  ],
  "memo": "Fourth transaction",
  "msg": {
    "type": "pos/Send",
    "value": {
      "amount": "10000000",
      "from_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
      "to_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
    }
  }
};

const exampleSend2 = {
  "chain_id": "testnet",
  "entropy": "-7780543831205109370",
  "fee": [
    {
      "amount": "2",
      "denom": "upokt"
    }
  ],
  "memo": "Fourth transaction",
  "msg": {
    "type": "pos/Send",
    "value": {
      "amount": "10203040",
      "from_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
      "to_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
    }
  }
};

const exampleSend3 = {
  "chain_id": "testnet",
  "entropy": "-7780543831205109370",
  "fee": [
    {
      "amount": "0000010000",
      "denom": "upokt"
    }
  ],
  "memo": "Fourth transaction",
  "msg": {
    "type": "pos/Send",
    "value": {
      "amount": "002000000000",
      "from_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
      "to_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
    }
  }
};

const exampleSend4 = {
  "chain_id": "testnet",
  "entropy": "-7780543831205109370",
  "fee": [
    {
      "amount": "12000",
      "denom": "upokt"
    }
  ],
  "memo": "Fourth transaction",
  "msg": {
    "type": "pos/Send",
    "value": {
      "amount": "10100000",
      "from_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
      "to_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
    }
  }
};

const exampleUnjail = {
  "chain_id": "testnet",
  "entropy": "-8051161335943327787",
  "fee": [
    {
      "amount": "10000",
      "denom": "upokt"
    }
  ],
  "memo": "",
  "msg": {
    "type": "pos/8.0MsgUnjail",
    "value": {
      "address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
      "signer_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8bb"
    }
  }
};

const exampleStake = {
  "chain_id": "testnet",
  "entropy": "2417661502575469960",
  "fee": [
    {
      "amount": "10000",
      "denom": "upokt"
    }
  ],
  "memo": "",
  "msg": {
    "type": "pos/8.0MsgStake",
    "value": {
      "chains": [
        "0034"
      ],
      "public_key": {
        "type": "crypto/ed25519_public_key",
        "value": "6b62a590bab42ea01383d3209fa719254977fb83624fbd6755d102264ba1adc0"
      },
      "service_url": "https://serviceURI.com:3000",
      "value": "1000000",
      "output_address":"db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
    }
  }
};

const exampleUnstake = {
  "chain_id": "testnet",
  "entropy": "-1105361304155186876",
  "fee": [
    {
      "amount": "10000",
      "denom": "upokt"
    }
  ],
  "memo": "",
  "msg": {
    "type": "pos/8.0MsgBeginUnstake",
    "value": {
      "signer_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8bb",
      "validator_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
    }
  }
};

describe("Signing tests", function() {
  it("can sign a simple transfer",
     testTransaction(
       "44'/635'/0/0",
       exampleSend,
       [
         {
           "header": "Transfer",
           "prompt": "POKT",
         },
         {
           "header": "From",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
           "paginate": true,
         },
         {
           "header": "To",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
           "paginate": true,
         },
         {
           "header": "Amount",
           "prompt": "POKT 10.0",
         },
         {
           "header": "Fee",
           "prompt": "POKT 0.012",
         },
         {
           "text": "Sign Transaction?",
           "x": 19,
           "y": 11
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         }
       ]
     ));
  it("can sign a simple transfer 2",
     testTransaction(
       "44'/635'/0/0",
       exampleSend2,
       [
         {
           "header": "Transfer",
           "prompt": "POKT",
         },
         {
           "header": "From",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
           "paginate": true,
         },
         {
           "header": "To",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
           "paginate": true,
         },
         {
           "header": "Amount",
           "prompt": "POKT 10.20304",
         },
         {
           "header": "Fee",
           "prompt": "POKT 0.000002",
         },
         {
           "text": "Sign Transaction?",
           "x": 19,
           "y": 11
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         }
       ]
     ));
  it("can sign a simple transfer, check decimal conversion",
     testTransaction(
       "44'/635'/0/0",
       exampleSend3,
       [
         {
           "header": "Transfer",
           "prompt": "POKT",
         },
         {
           "header": "From",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
           "paginate": true,
         },
         {
           "header": "To",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
           "paginate": true,
         },
         {
           "header": "Amount",
           "prompt": "POKT 2000.0",
         },
         {
           "header": "Fee",
           "prompt": "POKT 0.01",
         },
         {
           "text": "Sign Transaction?",
           "x": 19,
           "y": 11
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         }
]
     ));
  it("can sign a simple transfer, check decimal conversion 2",
     testTransaction(
       "44'/635'/0/0",
       exampleSend4,
       [
         {
           "header": "Transfer",
           "prompt": "POKT",
         },
         {
           "header": "From",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
           "paginate": true,
         },
         {
           "header": "To",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
           "paginate": true,
         },
         {
           "header": "Amount",
           "prompt": "POKT 10.1",
         },
         {
           "header": "Fee",
           "prompt": "POKT 0.012",
         },
         {
           "text": "Sign Transaction?",
           "x": 19,
           "y": 11
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         }
       ]
     ));
  it("can sign a simple unjail",
     testTransaction(
       "44'/635'/0/0",
       exampleUnjail,
       [
        {
          "header": "Unjail",
          "prompt": "Transaction"
        },
        {
          "header": "Address",
          "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
        },
        {
          "header": "Signer address",
          "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8bb"
        },
        {
          "header": "Fee",
          "prompt": "POKT 0.01",
        },
        {
          "text": "Sign Transaction?",
          "x": 19,
          "y": 11
        },
        {
          "text": "Confirm",
          "x": 43,
          "y": 11,
        }
       ]
       ));

  it("can sign a simple stake",
     testTransaction(
       "44'/635'/0/0",
       exampleStake,
       [
         {
           "header": "Stake",
           "prompt": "POKT",
         },
         {
           "header": "From",
           "prompt": "c2fc52e0bf6fa0686eb1b7afa8d6ab22d7138488",
           "paginate": true,
         },
         {
           "header": "Amount",
           "prompt": "POKT 1.0",
         },
         {
           "header": "Node Operator",
           "prompt": "6b62a590bab42ea01383d3209fa719254977fb83624fbd6755d102264ba1adc0 (crypto/ed25519_public_key)",
         },
         {
           "header": "Output Address",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
         },
         {
           "header": "Service URL",
           "prompt": "https://serviceURI.com:3000",
         },
         {
           "header": "Chain ID(s)",
           "prompt": "0034",
         },
         {
           "header": "Fee",
           "prompt": "POKT 0.01",
         },
         {
           "text": "Sign Transaction?",
           "x": 19,
           "y": 11
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
       ]

     ));

  it("can sign a simple unstake",
     testTransaction(
       "44'/635'/0/0",
       exampleUnstake,
       [
        {
          "header": "Unstake",
          "prompt": "POKT"
        },
        {
          "header": "Signer address",
          "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8bb"
        },
        {
          "header": "Unstake address",
          "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
        },
        {
          "header": "Fee",
          "prompt": "POKT 0.01",
        },
        {
          "text": "Sign Transaction?",
          "x": 19,
          "y": 11
        },
        {
          "text": "Confirm",
          "x": 43,
          "y": 11
        }
       ]
     ));
});

function testBlindSignFail(path: string, hash: string) {
  return async () => {
    await sendCommandExpectFail(
      async (client : Pokt) => {
        await client.blindSignTransaction(path, hash);
      });
  }
}

function testBlindSignFail2(path: string, hash: string) {
  return async () => {
    await sendCommandExpectFail(
      async (client : Pokt) => {
        // Enable and then disable
        await toggleBlindSigningSettings();
        await toggleBlindSigningSettings();
        await Axios.delete(BASE_URL + "/events");
        await client.blindSignTransaction(path, hash);
      });
  }
}

describe("Blind signing tests", function() {

  it("cannot sign arbitary JSON without settings enabled",
     testBlindSignFail(
       "44'/635'/0/0",
       '"ffd8cd79deb956fa3c7d9be0f836f20ac84b140168a087a842be4760e40e2b1c"'
     ));
  it("cannot sign abitrary JSON without settings enabled 2",
     testBlindSignFail2(
       "44'/635'/0/0",
       '"ffd8cd79deb956fa3c7d9be0f836f20ac84b140168a087a842be4760e40e2b1c"'
     ));

  it("can blind sign nonsense JSON",
     testBlindTransaction(
       "44'/635'/0/0",
       {
         foo: 1,
         bar: null,
       },
       [
         {
           "header": "WARNING",
           "prompt": "Blind Signing a Transaction is a very unusual operation. Do not continue unless you know what you are doing",
         },
         {
           "header": "Sign for Address",
           "prompt": "c2fc52e0bf6fa0686eb1b7afa8d6ab22d7138488",
         },
         {
           "text": "Blind Sign Transaction?",
           "x": 4,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         }
       ]
     ));
});
