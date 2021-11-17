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
      expect(rv.publicKey).to.equal("046f760e57383e3b5900f7c23b78a424e74bebbe9b7b46316da7c0b4b9c2c9301c0c076310eda30506141dd47c2d0a8a1d7ca2542482926ae23b781546193b9616");
      return;
    }, [
        {
          "text": "Provide Public Key",
          "x": 16,
          "y": 11
        },
        {
          "text": "pkh-CBB24246905B6BA63DB45BE62EDAEA0BEC58166BF39F9492CD199D5479686B2",
          "x": -48,
          "y": 11
        },
        {
          "text": "Confirm",
          "x": 43,
          "y": 11
        }
    ]);
  });
  
  it('provides a public key', async () => {
  await sendCommandAndAccept(async (kda : Kda) => {
      console.log("Started pubkey get");
      let rv = await kda.getPublicKey("0");
      console.log("Reached Pubkey Got");
      expect(rv.publicKey).to.equal("046f760e57383e3b5900f7c23b78a424e74bebbe9b7b46316da7c0b4b9c2c9301c0c076310eda30506141dd47c2d0a8a1d7ca2542482926ae23b781546193b9616");
      return;
    }, [
        {
          "text": "Provide Public Key",
          "x": 16,
          "y": 11
        },
        {
          "text": "pkh-CBB24246905B6BA63DB45BE62EDAEA0BEC58166BF39F9492CD199D5479686B2",
          "x": -48,
          "y": 11
        },
        {
          "text": "Confirm",
          "x": 43,
          "y": 11
        }
    ]);
  });

  it('runs a test', async () => { 
    
    await setAcceptAutomationRules();
    await Axios.delete("http://localhost:5000/events");

    let transport = await Transport.open("http://localhost:5000/apdu");
    let kda = new Kda(transport);
    
    let rv = await kda.getPublicKey("0/0");
   
    await Axios.post("http://localhost:5000/automation", {version: 1, rules: []});

    expect(rv.publicKey).to.equal("04e96341109fdba54691303553ee95b371d9745410f1090055fb7c0aa9e564445483f78cb81526e27ab7869fcd996eb8bd39add229b41f9e30bccccdc00a9d6c4c");
    expect(((await Axios.get("http://localhost:5000/events")).data["events"] as [any]).filter((a : any) => a["text"] != "W e l c o m e")).to.deep.equal([
        {
          "text": "Provide Public Key",
          "x": 16,
          "y": 11
        },
        {
          "text": "pkh-929B536E11497F4EF573A22680528E1785AEA757D9D3C29A5D4CDCBA9E2BF",
          "x": -50,
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

// These tests have been extracted mostly by the cosmos source code and the pokt
// proto files.

let exampleSend = {
    "account_number": "108",
    "chain_id": "cosmoshub-2",
    "fee": {
        "amount": [
            {
                "amount": "600",
                "denom": "uatom"
            }
        ],
        "gas": "200000"
    },
    "memo": "",
    "msgs": [
        {
            "type": "cosmos-sdk/MsgSend",
            "value": {
                "from_address": "cosmos1kky4yzth6gdrm8ga5zlfwhav33yr7hl87jycah",
                "to_address": "cosmosvaloper1kn3wugetjuy4zetlq6wadchfhvu3x740ae6z6x",
                "amount":[{"amount":"10","denom":"atom"}],
            }
        }
    ],
    "sequence": "106"
}

let exampleUnjail = {
    "account_number": "108",
    "chain_id": "cosmoshub-2",
    "fee": {
        "amount": [
            {
                "amount": "600",
                "denom": "uatom"
            }
        ],
        "gas": "200000"
    },
    "memo": "",
    "msgs": [
        {
            "type": "cosmos-sdk/MsgUnjail",
            "value": {
                "address": "cosmos1kky4yzth6gdrm8ga5zlfwhav33yr7hl87jycah",
            }
        }
    ],
    "sequence": "106"
}

let exampleStake = {
    "account_number": "108",
    "chain_id": "cosmoshub-2",
    "fee": {
        "amount": [
            {
                "amount": "600",
                "denom": "uatom"
            }
        ],
        "gas": "200000"
    },
    "memo": "",
    "msgs": [
        {
            "type": "cosmos-sdk/MsgStake",
            "value": {
                "public_key": "publicKey1",
                "chains": ["chain1", "chain2"],
                "value": "35",
                "service_url": "serviceurl1"
            }
        }
    ],
    "sequence": "106"
}

let exampleUnstake = {
    "account_number": "108",
    "chain_id": "cosmoshub-2",
    "fee": {
        "amount": [
            {
                "amount": "600",
                "denom": "uatom"
            }
        ],
        "gas": "200000"
    },
    "memo": "",
    "msgs": [
        {
            "type": "cosmos-sdk/MsgUnstake",
            "value": {
                "validator_address": "cosmos1kky4yzth6gdrm8ga5zlfwhav33yr7hl87jycah",
            }
        }
    ],
    "sequence": "106"
}

describe("Signing tests", function() {
  it.only("can sign a simple transfer",
     testTransaction(
       "0/0",
       JSON.stringify(exampleSend),
       [
         {
           "text": "Transfer from:",
           "x": 26,
           "y": 11,
         },
         {
           "text": "cosmos1kky4yzth",
           "x": 18,
           "y": 11,
         },
         {
           "text": "6gdrm8ga5zlfwha",
           "x": 18,
           "y": 11,
         },
         {
           "text": "v33yr7hl87jycah",
           "x": 22,
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
           "text": "cosmosvaloper1k",
           "x": 19,
           "y": 11,
         },
         {
           "text": "n3wugetjuy4zetl",
           "x": 20,
           "y": 11,
         },
         {
           "text": "q6wadchfhvu3x74",
           "x": 15,
           "y": 11,
         },
         {
           "text": "0ae6z6x",
           "x": 44,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "Amount:",
           "x": 42,
           "y": 11,
         },
         {
           "text": "10",
           "x": 58,
           "y": 11,
         },
         {
           "text": "Denom:",
           "x": 44,
           "y": 11,
         },
         {
           "text": "atom",
           "x": 51,
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
           "text": "1786E003E1DCE76D388108803846C1F0B4827A48BDF39F52C2D9506AF05903D2",
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
           "text": "pkh-929B536E11497F4EF50703A22680528E1785AEA757D9D3C29A5D4CDCBA9E02BF",
           "x": -50,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
       ]

       ));
  it.only("can sign a simple unjail",
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
               "text": "2E4A55FC71AA15D9C3CE02CCC03F3E4C50E00C6D298A5C8E6AB26D9A193A5450",
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
               "text": "pkh-929B536E11497F4EF50703A22680528E1785AEA757D9D3C29A5D4CDCBA9E02BF",
               "x": -50,
               "y": 11,
           },
           {
               "text": "Confirm",
               "x": 43,
               "y": 11,
           }
       ]));

  it.only("can sign a simple stake",
     testTransaction(
       "0/0",
       JSON.stringify(exampleStake),
       [
         {
           "text": "Stake with public key:",
           "x": 8,
           "y": 11,
         },
         {
           "text": "publicKey1",
           "x": 36,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "Chain:",
           "x": 48,
           "y": 11,
         },
         {
           "text": "chain1",
           "x": 47,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
         {
           "text": "Chain:",
           "x": 48,
           "y": 11,
         },
         {
           "text": "chain2",
           "x": 47,
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
           "text": "35",
           "x": 58,
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
           "text": "serviceurl1",
           "x": 37,
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
           "text": "E88C1414CD6E4E3F79FBFAD3FC60E41D77B88517304A828901E27FD6D6D89890",
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
           "text": "pkh-929B536E11497F4EF50703A22680528E1785AEA757D9D3C29A5D4CDCBA9E02BF",
           "x": -50,
           "y": 11,
         },
         {
           "text": "Confirm",
           "x": 43,
           "y": 11,
         },
       ]
     ));

  it.only("can sign a simple unstake",
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
           "text": "cosmos1kky4yzth",
           "x": 18,
           "y": 11,
         },
         {
           "text": "6gdrm8ga5zlfwha",
           "x": 18,
           "y": 11,
         },
         {
           "text": "v33yr7hl87jycah",
           "x": 22,
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
           "text": "707610613DA68A78AFBCEC4F8D256FEBFC1B262FACD3260CBD3EC52B145EDF59",
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
           "text": "pkh-929B536E11497F4EF50703A22680528E1785AEA757D9D3C29A5D4CDCBA9E02BF",
           "x": -50,
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
