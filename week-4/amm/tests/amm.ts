import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { Amm } from "../target/types/amm";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { expect } from "chai";

import { logTx } from "./helpers/log";
import { setupMintAndFund, getAta } from "./helpers/token";
import { deriveAllPdas, findConfigPda, findPoolPda } from "./pda";

describe("AMM", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.amm as Program<Amm>;
  const connection = provider.connection;
  const payer = (provider.wallet as anchor.Wallet).payer;

  const index = new BN(0);
  const fee = 30; // 0.3%

  let mintX: PublicKey;
  let mintY: PublicKey;
  let payerAtaX: PublicKey;
  let payerAtaY: PublicKey;

  // PDAs
  let config: PublicKey;
  let pool: PublicKey;
  let lpMint: PublicKey;
  let vaultX: PublicKey;
  let vaultY: PublicKey;

  // amounts
  const MINT_AMOUNT = 1_000_000_000; // 1000 tokens (6 decimals)
  const DEPOSIT_X = 100_000_000; // 100 tokens
  const DEPOSIT_Y = 200_000_000; // 200 tokens

  // Setup: create mints, fund payer, derive PDAs
  before(async () => {
    console.log("\n  --- Setup ---");
    console.log(`  Cluster : ${connection.rpcEndpoint}`);
    console.log(`  Payer   : ${payer.publicKey.toBase58()}`);

    // airdrop if localnet/surfpool
    if (!connection.rpcEndpoint.includes("devnet")) {
      const sig = await connection.requestAirdrop(
        payer.publicKey,
        2 * LAMPORTS_PER_SOL
      );
      await connection.confirmTransaction(sig, "confirmed");
      console.log(`  Airdrop : ${sig}\n`);
    }

    // create token X and fund payer
    const tokenX = await setupMintAndFund(connection, payer, MINT_AMOUNT);
    mintX = tokenX.mint;
    payerAtaX = tokenX.ata;

    // create token Y and fund payer
    const tokenY = await setupMintAndFund(connection, payer, MINT_AMOUNT);
    mintY = tokenY.mint;
    payerAtaY = tokenY.ata;

    // derive all PDAs
    const pdas = deriveAllPdas(index, mintX, mintY);
    config = pdas.config;
    pool = pdas.pool;
    lpMint = pdas.lpMint;
    vaultX = pdas.vaultX;
    vaultY = pdas.vaultY;

    console.log("  Mint X  :", mintX.toBase58());
    console.log("  Mint Y  :", mintY.toBase58());
    console.log("  Config  :", config.toBase58());
    console.log("  Pool    :", pool.toBase58());
    console.log("  LP Mint :", lpMint.toBase58());
    console.log("  Vault X :", vaultX.toBase58());
    console.log("  Vault Y :", vaultY.toBase58());
    console.log();
  });

  // 1. Initialize
  describe("Initialize", () => {
    it("creates a new pool with config", async () => {
      const tx = await program.methods
        .initialize(index, fee)
        .accountsPartial({
          payer: payer.publicKey,
          config,
          pool,
          mintX,
          mintY,
          lpMint,
          vaultX,
          vaultY,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await logTx(connection, tx, "Initialize pool");

      // verify config account
      const configAccount = await program.account.ammConfig.fetch(config);
      expect(configAccount.feeRate).to.equal(fee);
      expect(configAccount.index.toNumber()).to.equal(index.toNumber());
      expect(configAccount.authority.toBase58()).to.equal(
        payer.publicKey.toBase58()
      );

      // verify pool account
      const poolAccount = await program.account.pool.fetch(pool);
      expect(poolAccount.config.toBase58()).to.equal(config.toBase58());
      expect(poolAccount.mintX.toBase58()).to.equal(mintX.toBase58());
      expect(poolAccount.mintY.toBase58()).to.equal(mintY.toBase58());
      expect(poolAccount.locked).to.be.false;
    });

    it("fails with fee exceeding max (100 bps)", async () => {
      const failIndex = new BN(99);
      const [failConfig] = findConfigPda(failIndex);
      const [failPool] = findPoolPda(failConfig, mintX, mintY);
      let failLpMint = PublicKey.findProgramAddressSync(
        [Buffer.from("lp_mint"), failPool.toBuffer()],
        program.programId
      )[0];

      let failVautX = PublicKey.findProgramAddressSync(
        [Buffer.from("vault_x"), failPool.toBuffer(), mintX.toBuffer()],
        program.programId
      )[0];

      let failVautY = PublicKey.findProgramAddressSync(
        [Buffer.from("vault_y"), failPool.toBuffer(), mintY.toBuffer()],
        program.programId
      )[0];

      try {
        await program.methods
          .initialize(failIndex, 101) // fee > MAX_FEE (100)
          .accountsPartial({
            payer: payer.publicKey,
            config: failConfig,
            pool: failPool,
            mintX,
            mintY,
            lpMint: failLpMint,
            vaultX: failVautX,
            vaultY: failVautY,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        expect.fail("Should have thrown InvalidFee error");
      } catch (err: any) {
        expect(err.toString()).to.include("InvalidFee");
        console.log(
          "    [Expected error] InvalidFee:",
          err.error?.errorMessage || err.message
        );
      }
    });
  });

  // 2. Deposit
  describe("Deposit", () => {
    it("deposits liquidity and receives LP tokens", async () => {
      const payerLpAta = getAta(lpMint, payer.publicKey);

      const tx = await program.methods
        .deposit(
          new BN(DEPOSIT_X),
          new BN(DEPOSIT_Y),
          new BN(0) // min_lp = 0, accept any LP on first deposit
        )
        .accountsPartial({
          lpProvider: payer.publicKey,
          pool,
          mintX,
          mintY,
          lpMint,
          vaultX,
          vaultY,
          lpProviderAtaX: payerAtaX,
          lpProviderAtaY: payerAtaY,
          lpProviderLpAta: payerLpAta,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await logTx(connection, tx, "Deposit liquidity");

      // check vault balances
      const vaultXBalance = await connection.getTokenAccountBalance(vaultX);
      const vaultYBalance = await connection.getTokenAccountBalance(vaultY);
      expect(Number(vaultXBalance.value.amount)).to.equal(DEPOSIT_X);
      expect(Number(vaultYBalance.value.amount)).to.equal(DEPOSIT_Y);

      // check LP tokens minted
      const lpBalance = await connection.getTokenAccountBalance(payerLpAta);
      expect(Number(lpBalance.value.amount)).to.be.greaterThan(0);
      console.log(`    LP tokens received: ${lpBalance.value.uiAmountString}`);
    });

    it("fails depositing zero amounts", async () => {
      const payerLpAta = getAta(lpMint, payer.publicKey);

      try {
        await program.methods
          .deposit(new BN(0), new BN(0), new BN(0))
          .accountsPartial({
            lpProvider: payer.publicKey,
            pool,
            mintX,
            mintY,
            lpMint,
            vaultX,
            vaultY,
            lpProviderAtaX: payerAtaX,
            lpProviderAtaY: payerAtaY,
            lpProviderLpAta: payerLpAta,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        expect.fail("Should have thrown ZeroAmount error");
      } catch (err: any) {
        expect(err.toString()).to.include("ZeroAmount");
        console.log(
          "    [Expected error] ZeroAmount:",
          err.error?.errorMessage || err.message
        );
      }
    });
  });

  // 3. Swap
  describe("Swap", () => {
    it("swaps token X for token Y", async () => {
      const swapAmount = new BN(10_000_000); // 10 tokens

      const userAtaYBefore = await connection.getTokenAccountBalance(payerAtaY);

      const tx = await program.methods
        .swap(swapAmount, new BN(0), true) // x_to_y = true, min_out = 0
        .accountsPartial({
          user: payer.publicKey,
          config,
          pool,
          mintX,
          mintY,
          vaultX,
          vaultY,
          userAtaX: payerAtaX,
          userAtaY: payerAtaY,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await logTx(connection, tx, "Swap X -> Y");

      const userAtaYAfter = await connection.getTokenAccountBalance(payerAtaY);
      const yReceived =
        Number(userAtaYAfter.value.amount) -
        Number(userAtaYBefore.value.amount);
      expect(yReceived).to.be.greaterThan(0);
      console.log(`    Y tokens received: ${yReceived / 1e6}`);
    });

    it("swaps token Y for token X", async () => {
      const swapAmount = new BN(10_000_000); // 10 tokens

      const userAtaXBefore = await connection.getTokenAccountBalance(payerAtaX);

      const tx = await program.methods
        .swap(swapAmount, new BN(0), false) // x_to_y = false
        .accountsPartial({
          user: payer.publicKey,
          config,
          pool,
          mintX,
          mintY,
          vaultX,
          vaultY,
          userAtaX: payerAtaX,
          userAtaY: payerAtaY,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await logTx(connection, tx, "Swap Y -> X");

      const userAtaXAfter = await connection.getTokenAccountBalance(payerAtaX);
      const xReceived =
        Number(userAtaXAfter.value.amount) -
        Number(userAtaXBefore.value.amount);
      expect(xReceived).to.be.greaterThan(0);
      console.log(`    X tokens received: ${xReceived / 1e6}`);
    });

    it("fails swapping zero amount", async () => {
      try {
        await program.methods
          .swap(new BN(0), new BN(0), true)
          .accountsPartial({
            user: payer.publicKey,
            config,
            pool,
            mintX,
            mintY,
            vaultX,
            vaultY,
            userAtaX: payerAtaX,
            userAtaY: payerAtaY,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        expect.fail("Should have thrown ZeroAmount error");
      } catch (err: any) {
        expect(err.toString()).to.include("ZeroAmount");
        console.log(
          "    [Expected error] ZeroAmount:",
          err.error?.errorMessage || err.message
        );
      }
    });

    it("fails when slippage exceeded", async () => {
      try {
        await program.methods
          .swap(
            new BN(1_000_000), // 1 token in
            new BN(999_999_999), // min_out impossibly high
            true
          )
          .accountsPartial({
            user: payer.publicKey,
            config,
            pool,
            mintX,
            mintY,
            vaultX,
            vaultY,
            userAtaX: payerAtaX,
            userAtaY: payerAtaY,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        expect.fail("Should have thrown SlippageExceeded error");
      } catch (err: any) {
        expect(err.toString()).to.include("SlippageExceeded");
        console.log(
          "    [Expected error] SlippageExceeded:",
          err.error?.errorMessage || err.message
        );
      }
    });
  });

  // 4. Update Config
  describe("Update Config", () => {
    it("updates the fee rate", async () => {
      const newFee = 50; // 0.5%

      const tx = await program.methods
        .updateConfig(newFee, null, null, false)
        .accountsPartial({
          authority: payer.publicKey,
          config,
          pool,
        })
        .rpc();

      await logTx(connection, tx, "Update fee to 50 bps");

      const configAccount = await program.account.ammConfig.fetch(config);
      expect(configAccount.feeRate).to.equal(newFee);
    });

    it("locks the pool", async () => {
      const tx = await program.methods
        .updateConfig(null, true, null, false)
        .accountsPartial({
          authority: payer.publicKey,
          config,
          pool,
        })
        .rpc();

      await logTx(connection, tx, "Lock pool");

      const poolAccount = await program.account.pool.fetch(pool);
      expect(poolAccount.locked).to.be.true;
    });

    it("fails to swap on locked pool", async () => {
      try {
        await program.methods
          .swap(new BN(1_000_000), new BN(0), true)
          .accountsPartial({
            user: payer.publicKey,
            config,
            pool,
            mintX,
            mintY,
            vaultX,
            vaultY,
            userAtaX: payerAtaX,
            userAtaY: payerAtaY,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        expect.fail("Should have thrown PoolLocked error");
      } catch (err: any) {
        expect(err.toString()).to.include("PoolLocked");
        console.log(
          "    [Expected error] PoolLocked:",
          err.error?.errorMessage || err.message
        );
      }
    });

    it("fails to deposit on locked pool", async () => {
      const payerLpAta = getAta(lpMint, payer.publicKey);

      try {
        await program.methods
          .deposit(new BN(1_000_000), new BN(2_000_000), new BN(0))
          .accountsPartial({
            lpProvider: payer.publicKey,
            pool,
            mintX,
            mintY,
            lpMint,
            vaultX,
            vaultY,
            lpProviderAtaX: payerAtaX,
            lpProviderAtaY: payerAtaY,
            lpProviderLpAta: payerLpAta,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        expect.fail("Should have thrown PoolLocked error");
      } catch (err: any) {
        expect(err.toString()).to.include("PoolLocked");
        console.log(
          "    [Expected error] PoolLocked:",
          err.error?.errorMessage || err.message
        );
      }
    });

    it("unlocks the pool", async () => {
      const tx = await program.methods
        .updateConfig(null, false, null, false)
        .accountsPartial({
          authority: payer.publicKey,
          config,
          pool,
        })
        .rpc();

      await logTx(connection, tx, "Unlock pool");

      const poolAccount = await program.account.pool.fetch(pool);
      expect(poolAccount.locked).to.be.false;
    });

    it("fails when unauthorized user updates config", async () => {
      const faker = Keypair.generate();

      // fund the faker for tx fees
      const airdropSig = await connection.requestAirdrop(
        faker.publicKey,
        LAMPORTS_PER_SOL
      );
      await connection.confirmTransaction(airdropSig, "confirmed");

      try {
        await program.methods
          .updateConfig(10, null, null, false)
          .accountsPartial({
            authority: faker.publicKey,
            config,
            pool,
          })
          .signers([faker])
          .rpc();

        expect.fail("Should have thrown Unauthorized error");
      } catch (err: any) {
        expect(err.toString()).to.include("Unauthorized");
        console.log(
          "    [Expected error] Unauthorized:",
          err.error?.errorMessage || err.message
        );
      }
    });
  });

  // 5. Withdraw
  describe("Withdraw", () => {
    it("withdraws liquidity by burning LP tokens", async () => {
      const payerLpAta = getAta(lpMint, payer.publicKey);

      // get current LP balance
      const lpBalanceBefore = await connection.getTokenAccountBalance(
        payerLpAta
      );
      const lpAmount = Math.floor(Number(lpBalanceBefore.value.amount) / 2); // withdraw half

      const userXBefore = await connection.getTokenAccountBalance(payerAtaX);
      const userYBefore = await connection.getTokenAccountBalance(payerAtaY);

      const tx = await program.methods
        .withdraw(new BN(lpAmount), new BN(0), new BN(0))
        .accountsPartial({
          user: payer.publicKey,
          pool,
          mintX,
          mintY,
          lpMint,
          vaultX,
          vaultY,
          userAtaX: payerAtaX,
          userAtaY: payerAtaY,
          userLpAta: payerLpAta,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      await logTx(connection, tx, "Withdraw liquidity");

      const userXAfter = await connection.getTokenAccountBalance(payerAtaX);
      const userYAfter = await connection.getTokenAccountBalance(payerAtaY);

      const xReceived =
        Number(userXAfter.value.amount) - Number(userXBefore.value.amount);
      const yReceived =
        Number(userYAfter.value.amount) - Number(userYBefore.value.amount);

      expect(xReceived).to.be.greaterThan(0);
      expect(yReceived).to.be.greaterThan(0);

      console.log(`    X received back: ${xReceived / 1e6}`);
      console.log(`    Y received back: ${yReceived / 1e6}`);

      // LP balance should decrease
      const lpBalanceAfter = await connection.getTokenAccountBalance(
        payerLpAta
      );
      expect(Number(lpBalanceAfter.value.amount)).to.be.lessThan(
        Number(lpBalanceBefore.value.amount)
      );
    });

    it("fails withdrawing zero LP tokens", async () => {
      const payerLpAta = getAta(lpMint, payer.publicKey);

      try {
        await program.methods
          .withdraw(new BN(0), new BN(0), new BN(0))
          .accountsPartial({
            user: payer.publicKey,
            pool,
            mintX,
            mintY,
            lpMint,
            vaultX,
            vaultY,
            userAtaX: payerAtaX,
            userAtaY: payerAtaY,
            userLpAta: payerLpAta,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        expect.fail("Should have thrown ZeroLpAmount error");
      } catch (err: any) {
        expect(err.toString()).to.include("ZeroLpAmount");
        console.log(
          "    [Expected error] ZeroLpAmount:",
          err.error?.errorMessage || err.message
        );
      }
    });
  });
});
