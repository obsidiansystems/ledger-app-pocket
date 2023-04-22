import { sendCommandAndAccept, BASE_URL } from "./common";
import { expect } from 'chai';
import { describe, it } from 'mocha';
import Axios from 'axios';
import Pokt from "hw-app-pokt";

describe('public key tests', () => {

  afterEach( async function() {
    await Axios.post(BASE_URL + "/automation", {version: 1, rules: []});
    await Axios.delete(BASE_URL + "/events");
    // await (new Promise((resolve) => setTimeout(() => resolve(0), 1000)));
  });

  it('provides a public key', async () => {

    await sendCommandAndAccept(async (pokt : Pokt) => {
      const rv = await pokt.getPublicKey("44'/635'/0");
      expect(new Buffer(rv.publicKey).toString('hex')).to.equal("5a354b0d33de0006376dcb756113ab0fc3dc6e758101bcc9be5b7b538d5ae388");
      expect(new Buffer(rv.address).toString('hex')).to.equal("80e004848cd91888257d10e783420e923709e2d1");
      return;
    }, []);
  });

  it('does address verification', async () => {

    await sendCommandAndAccept(async (client : Pokt) => {
      const rv = await client.verifyAddress("44'/635'/0");
      expect(new Buffer(rv.publicKey).toString('hex')).to.equal("5a354b0d33de0006376dcb756113ab0fc3dc6e758101bcc9be5b7b538d5ae388");
      expect(new Buffer(rv.address).toString('hex')).to.equal("80e004848cd91888257d10e783420e923709e2d1");
      return;
    }, [
      {
        "header": "Provide Public Key",
        "prompt": "",
      },
      {
        "header": "Address",
        "prompt": "80e004848cd91888257d10e783420e923709e2d1",
      },
      {
        "text": "Confirm",
        "x": "<patched>",
        "y": "<patched>",
      },
    ]);
  });
});
