import { sendCommandAndAccept, BASE_URL } from "./common";
import { expect } from 'chai';
import { describe, it } from 'mocha';
import Axios from 'axios';
import Pokt from "hw-app-pokt";
import * as ed from 'noble-ed25519';

describe('basic tests', () => {

  afterEach( async function() {
    await Axios.post(BASE_URL + "/automation", {version: 1, rules: []});
    await Axios.delete(BASE_URL + "/events");
    // await (new Promise((resolve) => setTimeout(() => resolve(0), 1000)));
  });

  it('provides a public key', async () => {

    await sendCommandAndAccept(async (pokt : Pokt) => {
      let rv = await pokt.getPublicKey("44'/635'/0");
      expect(rv.publicKey).to.equal("5a354b0d33de0006376dcb756113ab0fc3dc6e758101bcc9be5b7b538d5ae388");
      return;
    }, []);
  });

  it('provides a public key', async () => {
  await sendCommandAndAccept(async (client : Pokt) => {
      let rv = await client.getPublicKey("44'/635'/0");
      expect(rv.publicKey).to.equal("5a354b0d33de0006376dcb756113ab0fc3dc6e758101bcc9be5b7b538d5ae388");
      return;
    }, []);
  });
});

function testTransaction(path: string, txn: string, prompts: any[]) {
     return async () => {
       let sig = await sendCommandAndAccept(
         async (client : Pokt) => {

           let pk = await client.getPublicKey(path);

           // We don't want the prompts from getPublicKey in our result
           await Axios.delete(BASE_URL + "/events");

           let sig = await client.signTransaction(path, Buffer.from(txn, "utf-8").toString("hex"));

           expect(await ed.verify(sig.signature, Buffer.from(txn, "utf-8"), pk.publicKey) ? "Signature Valid": "Signature Invalid").to.equal("Signature Valid");
         }, prompts);
     }
}

// These tests have been extracted interacting with the testnet via the cli.

let exampleSend = {
  "chain_id": "testnet",
  "entropy": "-7780543831205109370",
  "fee": [
    {
      "amount": "10000",
      "denom": "upokt"
    }
  ],
  "memo": "Fourth transaction",
  "msg": {
    "type": "pos/Send",
    "value": {
      "amount": "1000000",
      "from_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
      "to_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
    }
  }
};

let exampleUnjail = {
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

let exampleStake = {
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

let exampleUnstake = {
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
       JSON.stringify(exampleSend),
[
         {
        "header": "Signing",
        "prompt": "Transaction",
         },
         {
        "header": "For Account",
        "prompt": "c2fc52e0bf6fa0686eb1b7afa8d6ab22d7138488"
         },
         {
        "header": "Send",
        "prompt": "Transaction",
         },
         {
        "header": "Value",
        "prompt": "1000000",
         },
         {
        "header": "Transfer from",
        "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
         },
         {
        "header": "Transfer To",
        "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
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
       JSON.stringify(exampleUnjail),
       [
        { "header": "Signing",
          "prompt": "Transaction"
        },
        {
          "header": "For Account",
          "prompt": "c2fc52e0bf6fa0686eb1b7afa8d6ab22d7138488"
        },
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
           "text": "Confirm",
           "x": 43,
           "y": 11,
         }
       ]
       ));

  it("can sign a simple stake",
     testTransaction(
       "44'/635'/0/0",
       JSON.stringify(exampleStake),
       [
         {
           "header": "Signing",
          "prompt": "Transaction"
         },
         {
           "header": "For Account",
           "prompt": "c2fc52e0bf6fa0686eb1b7afa8d6ab22d7138488"
         },
         {
           "header": "Stake",
           "prompt": "Transaction",
         },
         {
           "header": "Chain",
           "prompt": "0034",
         },
         {
           "header": "Public Key",
           "prompt": "6b62a590bab42ea01383d3209fa719254977fb83624fbd6755d102264ba1adc0 (crypto/ed25519_public_key)",
         },
         {
           "header": "Service URL",
           "prompt": "https://serviceURI.com:3000",
         },
         {
           "header": "Value",
           "prompt": "1000000",
         },
         {
           "header": "Output Address",
           "prompt": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba",
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
       JSON.stringify(exampleUnstake),
       [
        { "header": "Signing",
          "prompt": "Transaction"
        },
        {
          "header": "For Account",
          "prompt": "c2fc52e0bf6fa0686eb1b7afa8d6ab22d7138488"
        },
        {
          "header": "Unstake",
          "prompt": "Transaction"
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
          "text": "Confirm",
          "x": 43,
          "y": 11
        }
       ]
     ));
});
