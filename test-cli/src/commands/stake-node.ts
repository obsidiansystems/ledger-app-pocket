import type { Arguments, CommandBuilder } from 'yargs';
// import Transport from '@ledgerhq/hw-transport-node-hid';
import Speculos from '@ledgerhq/hw-transport-node-speculos';
import { Common } from 'hw-app-obsidian-common';
import { Pocket, Configuration, HttpRpcProvider, CoinDenom, typeGuard, TransactionSignature, ITransactionSender } from '@pokt-network/pocket-js';
import { buildTxSender } from '../util';


type Options = {
  path: string;
  speculos: boolean;
  verbose: boolean;
  nodePubKey: string;
  chains: string;
  amount: string;
  serviceURL: string;
  fee: string;
  chainID: string;
  address: string,
  transactionSender: ITransactionSender;
};

export const command: string = 'stake-node <path> <nodePubKey> <chains> <amount> <serviceURL>';
export const desc: string = 'Stake a node';

const emptyExcl : any = {};

export const builder: CommandBuilder<Options, Options> = (yargs) =>
  yargs
    .options({
      speculos: {type: 'boolean'},
      verbose: {type: 'boolean'},
      chainID: {type: 'string'},
      memo: {type: 'string'},
      fee: {type: 'string'}
    })
    .describe({
             speculos: "Connect to a speculos instance instead of a real ledger; use --apdu 5555 when running speculos to enable.",
             verbose: "Print verbose output of message transfer with ledger",
             fee: "Override fee to given value",
             chainID: "Chain ID for transaction",
             memo: "Set memo for transaction"
    })
    .default('speculos', false)
    .default('verbose', false)
    .default('chainID', 'testnet')
    .middleware([ buildTxSender ])
    .default('fee', '10000')
    .positional('path', {type: 'string', demandOption: true, description: "Bip32 path to for the key to sign with."})
    .positional('nodePubKey', {type: 'string', demandOption: true, description: "Node public key"})
    .positional('chains', {type: 'string', demandOption: true, description: "Chain IDs (comma separated)"})
    .positional('amount', {type: 'string', demandOption: true, description: "Amount of funds to stake"})
    .positional('serviceURL', {type: 'string', demandOption: true, description: "Service URL"})
    ;

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  const { nodePubKey, chains, amount, serviceURL, fee, path, transactionSender, chainID } = argv;
  let rawTxResponse = await transactionSender
    .nodeStake(nodePubKey, chains.split(','), amount, new URL(serviceURL))
    .submit(chainID, fee);

  if(typeGuard(rawTxResponse, Error)) {
    throw rawTxResponse;
  }

  process.stdout.write(rawTxResponse.hash+"\n");
  process.exit(0);
}

