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
  nodeAddress: string;
  fee: string;
  chainID: string;
  address: string,
  transactionSender: ITransactionSender;
};

export const command: string = 'unjail-node <path> <nodeAddress>';
export const desc: string = 'Unjail a node';

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
    .positional('address', {type: 'string', demandOption: true, description: "Address of node to unstake"})
    ;

export const handler = async (argv: Arguments<Options>): Promise<void> => {
  const { fee, path, nodeAddress, transactionSender, chainID } = argv;
  let rawTxResponse = await transactionSender
    .nodeUnjail(nodeAddress)
    .submit(chainID, fee);

  if(typeGuard(rawTxResponse, Error)) {
    throw rawTxResponse;
  }

  process.stdout.write(rawTxResponse.hash+"\n");
  process.exit(0);
}

