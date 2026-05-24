import { Connection } from "@solana/web3.js";

/**
 * Logs a transaction signature with a descriptive message.
 * Confirms the transaction before logging.
 */
export async function logTx(
  connection: Connection,
  signature: string,
  message: string
): Promise<string> {
  const latestBlockhash = await connection.getLatestBlockhash();
  await connection.confirmTransaction(
    { signature, ...latestBlockhash },
    "confirmed"
  );

  const clusterUrl = connection.rpcEndpoint;
  let explorerBase: string;

  if (clusterUrl.includes("devnet")) {
    explorerBase = `https://explorer.solana.com/tx/${signature}?cluster=devnet`;
  } else {
    // surfpool / localnet — no explorer, just log raw sig
    explorerBase = signature;
  }

  console.log(`    [${message}]`);
  console.log(`      Signature : ${signature}`);
  console.log(`      Explorer  : ${explorerBase}`);
  console.log();

  return signature;
}
