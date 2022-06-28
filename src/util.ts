import type { Arguments, CommandBuilder } from 'yargs';
import Transport from '@ledgerhq/hw-transport-node-hid';
import Speculos from '@ledgerhq/hw-transport-node-speculos';
import { Common } from 'hw-app-obsidian-common';
import { Pocket, Configuration, HttpRpcProvider, CoinDenom, typeGuard, TransactionSignature, ITransactionSender } from '@pokt-network/pocket-js';


type Options = {
  [x: string]: unknown;
  path: string;
  speculos: boolean;
  verbose: boolean;
};
type ResultOptions = {
  [x: string]: unknown;
  address: string;
  transactionSender: ITransactionSender;
};

export const buildTxSender = async (argv: Arguments<Options>): Promise<void> => {
  const { amount, fee, path, speculos, to, verbose, chainID } = argv;
  let payloadString = argv.payload;

  const maxDispatchers = 5;
  const maxSessions = 2000;
  const requestTimeOut = 100000;
  // const dispatchers = "https://localhost:26657/,https://node1.testnet.pokt.network,https://node2.testnet.pokt.network,https://node3.testnet.pokt.network,https://node4.testnet.pokt.network,https://node5.testnet.pokt.network".split(",").map(x => new URL(x));
  const dispatchers = "http://localhost:8081/".split(",").map(x => new URL(x));

  const configuration = new Configuration(maxDispatchers, maxSessions, undefined, requestTimeOut, undefined, undefined, undefined, undefined, undefined, undefined, false);
  const rpcProvider = new HttpRpcProvider(dispatchers[0]);

  const pocket = new Pocket(dispatchers, rpcProvider, configuration);

  // Get address and public key
  
  let transport;
  if (speculos) {
    transport = await Speculos.open({apduPort: 5555});
  } else {
    transport = await Transport.open(undefined);
  }
  let app = new Common(transport, "", "", verbose === true);
  app.sendChunks = app.sendWithBlocks;

  let { publicKey, address } = await app.getPublicKey(path);

  if(address === null) {
    console.log("Did not receive an address");
    throw new Error("Did not receive an address");
  }

  let signer = async (payload: Buffer) => {
    let sig = await app.signTransaction(path, payload);
    if(verbose) {
      console.log("Transaction: ", payload.toString());
      console.log("Signature: ", sig.signature);
    }
    return new TransactionSignature(Buffer.from(publicKey, 'hex'), Buffer.from(sig.signature, 'hex'));
  };
  
  let transactionSender = pocket.withTxSigner(signer);

  if(typeGuard(transactionSender, Error)) {
    console.log("Error constructing transaction sender");
    throw transactionSender;
  }

  argv.address = address;
  argv.transactionSender = transactionSender;

}

