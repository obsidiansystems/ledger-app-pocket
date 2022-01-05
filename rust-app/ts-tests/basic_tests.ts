import { expect } from 'chai';
import { describe, it } from 'mocha';
import SpeculosTransport from '@ledgerhq/hw-transport-node-speculos';
import Axios from 'axios';
import Transport from "./common";
import Kda from "hw-app-pokt";

let setAcceptAutomationRules = async function() {
    await Axios.post("http://localhost:5000/automation", {
      version: 1,
      rules: [
        { "text": "W e l c o m e", "actions": [] },
        { "text": "Confirm", "actions": [ [ "button", 1, true ], [ "button", 2, true ], [ "button", 2, false ], [ "button", 1, false ] ]},
        { "actions": [ [ "button", 2, true ], [ "button", 2, false ] ]}
      ]
    });
}

let sendCommandAndAccept = async function(command : any, prompts : any) {
    await setAcceptAutomationRules();
    await Axios.delete("http://localhost:5000/events");

    let transport = await Transport.open("http://localhost:5000/apdu");
    let kda = new Kda(transport);
    
    //await new Promise(resolve => setTimeout(resolve, 100));
    
    await command(kda);
    
    //await new Promise(resolve => setTimeout(resolve, 100));


    expect(((await Axios.get("http://localhost:5000/events")).data["events"] as [any]).filter((a : any) => a["text"] != "W e l c o m e")).to.deep.equal(prompts);
}

describe('basic tests', () => {
  afterEach( async function() {
    console.log("Clearing settings");
    await Axios.post("http://localhost:5000/automation", {version: 1, rules: []});
    await Axios.delete("http://localhost:5000/events");
  });

  it('provides a public key', async () => {

    await sendCommandAndAccept(async (kda : Kda) => {
      console.log("Started pubkey get");
      let rv = await kda.getPublicKey("0");
      console.log("Reached Pubkey Got");
      expect(rv.publicKey).to.equal("026f760e57383e3b5900f7c23b78a424e74bebbe9b7b46316da7c0b4b9c2c9301c");
      return;
    }, [
      {
        "text": "Provide Public Key",
        "x": 16,
        "y": 11,
      },
      {
        "text": "pkh-09CB550E56C3B91B1AB9F7836288641BC99A3C2B647470768B86C8D85863480F",
        "x": -49,
        "y": 11,
      },
      {
        "text": "Confirm",
        "x": 43,
        "y": 11,
      },
    ]);
  });
  
  it('provides a public key', async () => {
  await sendCommandAndAccept(async (kda : Kda) => {
      console.log("Started pubkey get");
      let rv = await kda.getPublicKey("0");
      console.log("Reached Pubkey Got");
      expect(rv.publicKey).to.equal("026f760e57383e3b5900f7c23b78a424e74bebbe9b7b46316da7c0b4b9c2c9301c");
      return;
    },
    [
      {
        "text": "Provide Public Key",
        "x": 16,
        "y": 11,
      },
      {
        "text": "pkh-09CB550E56C3B91B1AB9F7836288641BC99A3C2B647470768B86C8D85863480F",
        "x": -49,
        "y": 11,
      },
      {
        "text": "Confirm",
        "x": 43,
        "y": 11,
      },
    ]);
  });

  it('runs a test', async () => {
    
    await setAcceptAutomationRules();
    await Axios.delete("http://localhost:5000/events");

    let transport = await Transport.open("http://localhost:5000/apdu");
    let kda = new Kda(transport);
    
    let rv = await kda.getPublicKey("0/0");
   
    await Axios.post("http://localhost:5000/automation", {version: 1, rules: []});

    expect(rv.publicKey).to.equal("02e96341109fdba54691303553ee95b371d9745410f1090055fb7c0aa9e5644454");
    expect(((await Axios.get("http://localhost:5000/events")).data["events"] as [any]).filter((a : any) => a["text"] != "W e l c o m e")).to.deep.equal([
        {
          "text": "Provide Public Key",
          "x": 16,
          "y": 11
        },
        {
          "text": "pkh-493E8E5DBDF933EDD1495A4E304EC8B8155312BBBE66A1783A03DF9F6B5500C7",
          "x": -47,
          "y": 11
        },
        {
          "text": "Confirm",
          "x": 43,
          "y": 11
        }
    ]);
    await Axios.delete("http://localhost:5000/events");
  });
});

function testTransaction(path: string, txn: string, prompts: any[]) {
     return async () => {
       await sendCommandAndAccept(
         async (kda : Kda) => {
           console.log("Started pubkey get");
           let rv = await kda.signTransaction(path, Buffer.from(txn, "utf-8").toString("hex"));
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
}

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
    "type": "pos/MsgUnjail",
    "value": {
      "address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
    }
  }
}

let exampleStake =
  {
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
      "type": "pos/MsgStake",
      "value": {
        "chains": [
          "0034"
        ],
        "public_key": {
          "type": "crypto/ed25519_public_key",
          "value": "6b62a590bab42ea01383d3209fa719254977fb83624fbd6755d102264ba1adc0"
        },
        "service_url": "https://serviceURI.com:3000",
        "value": "1000000"
      }
    }
  }

let exampleUnstake =
  {
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
      "type": "pos/MsgBeginUnstake",
      "value": {
        "validator_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
      }
    }
  }

describe("Signing tests", function() {
  it("can sign a simple transfer",
     testTransaction(
       "0/0",
       JSON.stringify(exampleSend),
[
  {
    "text": "Value:",
    "x": 48,
    "y": 11,
  },
  {
    "text": "1000000",
    "x": 43,
    "y": 11,
  },
  {
    "text": "Confirm",
    "x": 43,
    "y": 11,
  },
  {
    "text": "Transfer from:",
    "x": 26,
    "y": 11,
  },
  {
    "text": "db987ccfa2a71b2",
    "x": 19,
    "y": 11,
  },
  {
    "text": "ec9a56c88c77a7c",
    "x": 21,
    "y": 11,
  },
  {
    "text": "f66d01d8ba",
    "x": 33,
    "y": 11,
  },
  {
    "text": "Confirm",
    "x": 43,
    "y": 11,
  },
  {
    "text": "Transfer to:",
    "x": 34,
    "y": 11,
  },
  {
    "text": "db987ccfa2a71b2",
    "x": 19,
    "y": 11,
  },
  {
    "text": "ec9a56c88c77a7c",
    "x": 21,
    "y": 11,
  },
  {
    "text": "f66d01d8ba",
    "x": 33,
    "y": 11,
  },
  {
    "text": "Confirm",
    "x": 43,
    "y": 11,
  },
  {
    "text": "Sign Hash?",
    "x": 36,
    "y": 11,
  },
  {
    "text": "D9779BB631C0BA7A991D5E6166B6419F5557CB423FD137079121986607856D92",
    "x": -47,
    "y": 11,
  },
  {
    "text": "Confirm",
    "x": 43,
    "y": 11,
  },
  {
    "text": "With PKH",
    "x": 40,
    "y": 11,
  },
  {
    "text": "pkh-493E8E5DBDF933EDD1495A4E304EC8B8155312BBBE66A1783A03DF9F6B5500C7",
    "x": -47,
    "y": 11,
  },
  {
    "text": "Confirm",
    "x": 43,
    "y": 11,
  },
]
     ));
  it("can sign a simple unjail",
     testTransaction(
       "0/0",
       JSON.stringify(exampleUnjail),
       [
         {
           "text": "Sign Hash?",
           "x": 36,
           "y": 11,
         },
         {
           "text": "FF11A8FD314B73EE4EB15D7097F2CAB8E0A4896427E5384254A47B3F1AB022FD",
           "x": -48,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "With PKH",
           "x": 40,
           "y": 11,
         },
         {
           "text": "pkh-493E8E5DBDF933EDD1495A4E304EC8B8155312BBBE66A1783A03DF9F6B5500C7",
           "x": -47,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
       ]
       ));

  it("can sign a simple stake",
     testTransaction(
       "0/0",
       JSON.stringify(exampleStake),
       [
         {
           "text": "Chain:",
           "x": 48,
           "y": 11,
         },
         {
           "text": "0034",
           "x": 51,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "Type: ",
           "x": 48,
           "y": 11,
         },
         {
           "text": "crypto/ed25519_",
           "x": 22,
           "y": 11,
         },
         {
           "text": "public_key",
           "x": 37,
           "y": 11,
         },
         {
           "text": "Value: ",
           "x": 47,
           "y": 11,
         },
         {
           "text": "6b62a590bab42ea",
           "x": 17,
           "y": 11,
         },
         {
           "text": "01383d3209fa719",
           "x": 19,
           "y": 11,
         },
         {
           "text": "254977fb83624fb",
           "x": 17,
           "y": 11,
         },
         {
           "text": "d6755d102264ba1",
           "x": 17,
           "y": 11,
         },
         {
           "text": "adc0",
           "x": 52,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "Service url:",
           "x": 36,
           "y": 11,
         },
         {
           "text": "https://service",
           "x": 28,
           "y": 11,
         },
         {
           "text": "URI.com:3000",
           "x": 29,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "Value:",
           "x": 48,
           "y": 11,
         },
         {
           "text": "1000000",
           "x": 43,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "Sign Hash?",
           "x": 36,
           "y": 11,
         },
         {
           "text": "9BF2A5EAAECA8A5FAD5C2C4CA0C2D3FFEABC28A2AF2FE337343136DBEFF4437F",
           "x": -48,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "With PKH",
           "x": 40,
           "y": 11,
         },
         {
           "text": "pkh-493E8E5DBDF933EDD1495A4E304EC8B8155312BBBE66A1783A03DF9F6B5500C7",
           "x": -47,
           "y": 11,
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
       "0/0",
       JSON.stringify(exampleUnstake),
       [
         {
           "text": "Transfer from:",
           "x": 26,
           "y": 11,
         },
         {
           "text": "db987ccfa2a71b2",
           "x": 19,
           "y": 11,
         },
         {
           "text": "ec9a56c88c77a7c",
           "x": 21,
           "y": 11,
         },
         {
           "text": "f66d01d8ba",
           "x": 33,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "Sign Hash?",
           "x": 36,
           "y": 11,
         },
         {
           "text": "BAF8E9CB74DF4DBD4B28E1A6B77A472E371C8ABC091EC606A5810A944F8F3851",
           "x": -49,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "With PKH",
           "x": 40,
           "y": 11,
         },
         {
           "text": "pkh-493E8E5DBDF933EDD1495A4E304EC8B8155312BBBE66A1783A03DF9F6B5500C7",
           "x": -47,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
       ]

     ));
});
