import { expect } from 'chai';
import { describe, it } from 'mocha';
import SpeculosTransport from '@ledgerhq/hw-transport-node-speculos';
import Axios from 'axios';
import Transport from "./common";
import Pokt from "hw-app-pokt";

let ignoredScreens = [ "W e l c o m e", "Cancel", "Working...", "Exit", "Pokt 0.2.0"]

let setAcceptAutomationRules = async function() {
    await Axios.post("http://localhost:5000/automation", {
      version: 1,
      rules: [
        ... ignoredScreens.map(txt => { return { "text": txt, "actions": [] } }),
        { "y": 16, "actions": [] },
        { "text": "Confirm", "actions": [ [ "button", 1, true ], [ "button", 2, true ], [ "button", 2, false ], [ "button", 1, false ] ]},
        { "actions": [ [ "button", 2, true ], [ "button", 2, false ] ]}
      ]
    });
}

let processPrompts = function(prompts: [any]) {
  let i = prompts.filter((a : any) => !ignoredScreens.includes(a["text"])).values();
  let {done, value} = i.next();
  let header = "";
  let prompt = "";
  let rv = [];
  while(!done) {
    if(value["y"] == 1) {
      if(value["text"] != header) {
        if(header || prompt) rv.push({ header, prompt });
        header = value["text"];
        prompt = "";
      }
    } else if(value["y"] == 16) {
      prompt += value["text"];
    } else {
      if(header || prompt) rv.push({ header, prompt });
      rv.push(value);
      header = "";
      prompt = "";
    }
    ({done, value} = i.next());
  }
  return rv;
}

let sendCommandAndAccept = async function(command : any, prompts : any) {
    await setAcceptAutomationRules();
    await Axios.delete("http://localhost:5000/events");

    let transport = await Transport.open("http://localhost:5000/apdu");
    let kda = new Pokt(transport);
    
    //await new Promise(resolve => setTimeout(resolve, 100));
    
    let err = null;

    try { await command(kda); } catch(e) {
      err = e;
    }
    
    //await new Promise(resolve => setTimeout(resolve, 100));


    expect(processPrompts((await Axios.get("http://localhost:5000/events")).data["events"] as [any])).to.deep.equal(prompts);
    // expect(((await Axios.get("http://localhost:5000/events")).data["events"] as [any]).filter((a : any) => a["text"] != "W e l c o m e")).to.deep.equal(prompts);
    if(err) throw(err);
}

describe('basic tests', () => {
  afterEach( async function() {
    console.log("Clearing settings");
    await Axios.post("http://localhost:5000/automation", {version: 1, rules: []});
    await Axios.delete("http://localhost:5000/events");
  });

  it('provides a public key', async () => {

    await sendCommandAndAccept(async (pokt : Pokt) => {
      console.log("Started pubkey get");
      let rv = await pokt.getPublicKey("0");
      console.log("Reached Pubkey Got");
      expect(rv.publicKey).to.equal("026f760e57383e3b5900f7c23b78a424e74bebbe9b7b46316da7c0b4b9c2c9301c");
      return;
    }, [
      { "header": "Provide Public Key", "prompt": "pkh-09CB550E56C3B91B1AB9F7836288641BC99A3C2B647470768B86C8D85863480F" },
      {
        "text": "Confirm",
        "x": 43,
        "y": 11,
      },
    ]);
  });
  
  it('provides a public key', async () => {
  await sendCommandAndAccept(async (kda : Pokt) => {
      console.log("Started pubkey get");
      let rv = await kda.getPublicKey("0");
      console.log("Reached Pubkey Got");
      expect(rv.publicKey).to.equal("026f760e57383e3b5900f7c23b78a424e74bebbe9b7b46316da7c0b4b9c2c9301c");
      return;
    },
    [
      { "header": "Provide Public Key", "prompt": "pkh-09CB550E56C3B91B1AB9F7836288641BC99A3C2B647470768B86C8D85863480F" },
      {
        "text": "Confirm",
        "x": 43,
        "y": 11,
      },
    ]);
  });
});

function testTransaction(path: string, txn: string, prompts: any[]) {
     return async () => {
       await sendCommandAndAccept(
         async (kda : Pokt) => {
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
    "type": "pos/MsgUnjail",
    "value": {
      "address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
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
    "type": "pos/MsgBeginUnstake",
    "value": {
      "validator_address": "db987ccfa2a71b2ec9a56c88c77a7cf66d01d8ba"
    }
  }
};

describe("Signing tests", function() {
  it("can sign a simple transfer",
     testTransaction(
       "0/0",
       JSON.stringify(exampleSend),
[
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
        "header": "Sign Hash?",
        "prompt": "D9779BB631C0BA7A991D5E6166B6419F5557CB423FD137079121986607856D92",
         },
         {
        "header": "With PKH",
        "prompt": "pkh-493E8E5DBDF933EDD1495A4E304EC8B8155312BBBE66A1783A03DF9F6B5500C7",
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
       "0/0",
       JSON.stringify(exampleUnjail),
       [
         {
        "header": "Sign Hash?",
        "prompt": "FF11A8FD314B73EE4EB15D7097F2CAB8E0A4896427E5384254A47B3F1AB022FD",
         },
         {
        "header": "With PKH",
        "prompt": "pkh-493E8E5DBDF933EDD1495A4E304EC8B8155312BBBE66A1783A03DF9F6B5500C7",
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
       "0/0",
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
        "header": "Sign Hash?",
        "prompt": "9BF2A5EAAECA8A5FAD5C2C4CA0C2D3FFEABC28A2AF2FE337343136DBEFF4437F",
         },
         {
        "header": "With PKH",
        "prompt": "pkh-493E8E5DBDF933EDD1495A4E304EC8B8155312BBBE66A1783A03DF9F6B5500C7",
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
       []
     ));
});
