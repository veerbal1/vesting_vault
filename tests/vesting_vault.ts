import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { VestingVault } from "../target/types/vesting_vault";
import {
  createMint,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { expect } from "chai";
import BN from "bn.js";

describe("vesting_vault", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const provider = anchor.getProvider();
  const admin = provider.wallet;
  const beneficiary = anchor.web3.Keypair.generate();
  let tokenMint: anchor.web3.PublicKey;
  let adminTokenAccount: anchor.web3.PublicKey;
  let vaultPDA: anchor.web3.PublicKey;
  let vestingPDA: anchor.web3.PublicKey;

  before(async () => {
    let mint = await createMint(
      provider.connection,
      admin.payer,
      admin.publicKey,
      null,
      9
    );
    tokenMint = mint;

    // Create Admin Token Account
    let adminAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin.payer,
      tokenMint,
      admin.publicKey
    );
    adminTokenAccount = adminAccount.address;

    await mintTo(
      provider.connection,
      admin.payer,
      tokenMint,
      adminTokenAccount,
      admin.publicKey,
      500 * 10 ** 9
    );

    const [vaultPDAAddress] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vault_state")],
      program.programId
    );
    vaultPDA = vaultPDAAddress;

    const [vestingPDAAddress] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("vesting"), beneficiary.publicKey.toBuffer()],
      program.programId
    );
    vestingPDA = vestingPDAAddress;
  });

  const program = anchor.workspace.vestingVault as Program<VestingVault>;

  it("Initialize Vault!", async () => {
    // Add your test here.
    await program.methods
      .initializeVault()
      .accounts({
        admin: admin.publicKey,
        mint: tokenMint,
      })
      .rpc();

    const result = await program.account.vaultState.fetch(vaultPDA);

    expect(result.mint.toString()).to.be.equal(tokenMint.toString());
  });

  it("Initialize Vesting", async () => {
    const now = Math.floor(Date.now() / 1000);
    const end_at = new BN(now + 10); // 10 seconds for testing
    const cliff_period_till = new BN(now + 2); // Cliff after 2 seconds
    const total_totals = new BN(100).mul(new BN(10 ** 9));
    await program.methods
      .initializeVesting(beneficiary.publicKey, total_totals, end_at, cliff_period_till)
      .accounts({
        admin: admin.publicKey,
        adminTokenAccount: adminTokenAccount,
        mint: tokenMint,
      })
      .rpc();

    let vestingAccount = await program.account.vestingAccount.fetch(vestingPDA);
    expect(vestingAccount.beneficiary.toString()).to.be.equal(
      beneficiary.publicKey.toString()
    );
    expect(vestingAccount.totalTokens.toString()).to.be.equal(
      total_totals.toString()
    );
    // You can test startedAt by checking it's close to the current time, allowing for a few seconds of difference due to execution delay.
    const now_ts = Math.floor(Date.now() / 1000);
    const startedAt = parseInt(vestingAccount.startedAt.toString());
    expect(Math.abs(startedAt - now_ts)).to.be.lessThan(10);
    expect(vestingAccount.endAt.toString()).to.be.equal(end_at.toString());
    expect(vestingAccount.claimedTokens.toString()).to.be.equal("0");
    expect(vestingAccount.cliffPeriodTill.toString()).to.be.equal(
      cliff_period_till.toString()
    );
  });

  it("test claim", async () => {
    let benAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin.payer,
      tokenMint,
      beneficiary.publicKey
    );
    expect(benAccount.amount.toString()).to.be.equal("0");

    await new Promise((resolve) => setTimeout(resolve, 5000));

    await program.methods
      .claim()
      .accounts({
        mint: tokenMint,
        beneficiary: beneficiary.publicKey,
      }).signers([beneficiary])
      .rpc();

    let acc = await getAccount(provider.connection, benAccount.address);
    
    expect(Number(acc.amount)).to.be.greaterThan(40 * 10 ** 9);
    expect(Number(acc.amount)).to.be.lessThanOrEqual(100 * 10 ** 9);
  });
});
