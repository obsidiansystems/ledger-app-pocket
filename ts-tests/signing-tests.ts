import { VERSION, sendCommandAndAccept, BASE_URL, sendCommandExpectFail } from "./common";
import { expect } from 'chai';
import { describe, it } from 'mocha';
import Axios from 'axios';
import Pokt from "hw-app-pokt";
import * as ed from '@noble/ed25519';

function testTransaction(path: string, txn: string, prompts: any[]) {
     return async () => {
       await sendCommandAndAccept(
         async (client : Pokt) => {

           const pk = await client.getPublicKey(path);

           // We don't want the prompts from getPublicKey in our result
           await Axios.delete(BASE_URL + "/events");

           const sig = await client.signTransaction(path, Buffer.from(txn, "utf-8").toString("hex"));

           expect(await ed.verify(sig.signature, Buffer.from(txn, "utf-8"), pk.publicKey) ? "Signature Valid": "Signature Invalid").to.equal("Signature Valid");
         }, prompts);
     }
}

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
       JSON.stringify(exampleSend),
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
           "prompt": "10.0",
         },
         {
           "header": "Fees",
           "prompt": "0.012",
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
       JSON.stringify(exampleSend2),
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
           "prompt": "10.20304",
         },
         {
           "header": "Fees",
           "prompt": "0.000002",
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
       JSON.stringify(exampleSend3),
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
           "prompt": "2000.0",
         },
         {
           "header": "Fees",
           "prompt": "0.01",
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
       JSON.stringify(exampleSend4),
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
           "prompt": "10.1",
         },
         {
           "header": "Fees",
           "prompt": "0.012",
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
