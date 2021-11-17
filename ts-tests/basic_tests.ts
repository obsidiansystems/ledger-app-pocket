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
  it("can sign a different simple transfer",
     testTransaction(
       "0/0",
       '{"networkId":"mainnet01","payload":{"exec":{"data":{},"code":"(coin.transfer \"aab7d3e457f3f78480832d6ac4ace7387f460620a63a5b68c8c799d6bff1566a\" \"4c310df6224d674d80463a29cde00cb0ecfb71e0cfdce494243a61b8ea572dfd\" 2.0)"}},"signers":[{"pubKey":"aab7d3e457f3f78480832d6ac4ace7387f460620a63a5b68c8c799d6bff1566a","clist":[{"args":["aab7d3e457f3f78480832d6ac4ace7387f460620a63a5b68c8c799d6bff1566a","4c310df6224d674d80463a29cde00cb0ecfb71e0cfdce494243a61b8ea572dfd",2],"name":"coin.TRANSFER"},{"args":[],"name":"coin.GAS"}]}],"meta":{"creationTime":1634009195,"ttl":900,"gasLimit":600,"chainId":"0","gasPrice":1.0e-6,"sender":"aab7d3e457f3f78480832d6ac4ace7387f460620a63a5b68c8c799d6bff1566a"},"nonce":"\"2021-10-12T03:27:35.231Z\""}',
       []));
  it("can sign a transfer-create",
     testTransaction(
       "0/0",
       '{"networkId":"mainnet01","payload":{"exec":{"data":{"recp-ks":{"pred":"keys-all","keys":["875e4493e19c8721583bfb46f0768f10266ebcca33c4a0e04bc099a7044a90f7"]}},"code":"(coin.transfer-create \"e4a1b2980c086c4551ab7d2148cf56e9774c64eb86f795d5fd83e39ccfd2ec66\" \"875e4493e19c8721583bfb46f0768f10266ebcca33c4a0e04bc099a7044a90f7\" (read-keyset \"recp-ks\") 4.98340488)"}},"signers":[{"pubKey":"e4a1b2980c086c4551ab7d2148cf56e9774c64eb86f795d5fd83e39ccfd2ec66","clist":[{"args":[],"name":"coin.GAS"},{"args":["e4a1b2980c086c4551ab7d2148cf56e9774c64eb86f795d5fd83e39ccfd2ec66","875e4493e19c8721583bfb46f0768f10266ebcca33c4a0e04bc099a7044a90f7",4.98340488],"name":"coin.TRANSFER"}]}],"meta":{"creationTime":1634009142,"ttl":28800,"gasLimit":60000,"chainId":"0","gasPrice":1.0e-6,"sender":"e4a1b2980c086c4551ab7d2148cf56e9774c64eb86f795d5fd83e39ccfd2ec66"},"nonce":"\"1634009156943\""}',
       []));
  it("can sign a transfer-create",
     testTransaction(
       "0/0",
       '{"networkId":"mainnet01","payload":{"exec":{"data":{"recp-ks":{"pred":"keys-all","keys":["875e4493e19c8721583bfb46f0768f10266ebcca33c4a0e04bc099a7044a90f7"]}},"code":"(coin.transfer-create \"73580ffb3e5ca9859442395d4c1cb0bf3aa4e7246564ce943b7ae508b3ee7c03\" \"875e4493e19c8721583bfb46f0768f10266ebcca33c4a0e04bc099a7044a90f7\" (read-keyset \"recp-ks\") 4.89093455)"}},"signers":[{"pubKey":"73580ffb3e5ca9859442395d4c1cb0bf3aa4e7246564ce943b7ae508b3ee7c03","clist":[{"args":[],"name":"coin.GAS"},{"args":["73580ffb3e5ca9859442395d4c1cb0bf3aa4e7246564ce943b7ae508b3ee7c03","875e4493e19c8721583bfb46f0768f10266ebcca33c4a0e04bc099a7044a90f7",4.89093455],"name":"coin.TRANSFER"}]}],"meta":{"creationTime":1634009098,"ttl":28800,"gasLimit":60000,"chainId":"0","gasPrice":1.0e-6,"sender":"73580ffb3e5ca9859442395d4c1cb0bf3aa4e7246564ce943b7ae508b3ee7c03"},"nonce":"\"1634009113073\""}',
       []));

  it("can sign a rotate transaction",
     testTransaction(
       "0/0",
'{"networkId":"mainnet01","payload":{"exec":{"data":{"ks":{"pred":"keys-all","keys":["d3300d284f4bcfbc91555184ef026a356e57ff0fa97b5e6c9830750892cd3093"]}},"code":"(coin.rotate \"d3300d284f4bcfbc91555184ef026a356e57ff0fa97b5e6c9830750892cd3093\" (read-keyset \"ks\"))"}},"signers":[{"pubKey":"81b4511b257fb975dace13e823c257c17ac6a695da65f91b6036d6e1429268fc","clist":[{"args":[],"name":"coin.GAS"},{"args":["d3300d284f4bcfbc91555184ef026a356e57ff0fa97b5e6c9830750892cd3093"],"name":"coin.ROTATE"}]}],"meta":{"creationTime":1633466764,"ttl":28800,"gasLimit":1500,"chainId":"0","gasPrice":1.0e-5,"sender":"81b4511b257fb975dace13e823c257c17ac6a695da65f91b6036d6e1429268fc"},"nonce":"\"1633466764\""}',
       []));

});
