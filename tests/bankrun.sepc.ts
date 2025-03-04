import { describe, it } from 'node:test';
import { BN, Program } from '@coral-xyz/anchor';
import { BankrunProvider } from 'anchor-bankrun';
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { createAccount, createMint, mintTo } from 'spl-token-bankrun';
import { PythSolanaReceiver } from '@pythnetwork/pyth-solana-receiver';

import { startAnchor, BanksClient, ProgramTestContext } from 'solana-bankrun';

import { PublicKey, Keypair, Connection } from '@solana/web3.js';

// @ts-ignore
import IDL from '../target/idl/lending.json';
import { Lending } from '../target/types/lending';
import { BankrunContextWrapper } from '../bankrun_utils/bankrunConnection';

describe('Lending Smart Contract Tests', async () => {
  console.log('Starting tests...');
  let signer: Keypair;
  let usdcBankAccount: PublicKey;
  let solBankAccount: PublicKey;
  let UserUSDCTokenAccount: PublicKey;
  let solTokenAccount: PublicKey;
  let provider: BankrunProvider;
  let program: Program<Lending>;
  let banksClient: BanksClient;
  let context: ProgramTestContext;
  let bankrunContextWrapper: BankrunContextWrapper;

  const pyth = new PublicKey('7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE');


  const devnetConnection = new Connection('https://solana-devnet.g.alchemy.com/v2/u8NCITxCIZ_Hj4DOZ0DEII7kSJBctO-9');
  const accountInfo = await devnetConnection.getAccountInfo(pyth);

  context = await startAnchor(
    '',
    [{ name: 'lending', programId: new PublicKey(IDL.address) }],
    [
      {//预加载的账户信息,从 Devnet 上获取的 Pyth 价格预言机账户数据注入到测试环境中。
        address: pyth,
        info: accountInfo,
      },
    ]
  );

  provider = new BankrunProvider(context);
  bankrunContextWrapper = new BankrunContextWrapper(context);



  const connection = bankrunContextWrapper.connection.toConnection();

  const pythSolanaReceiver = new PythSolanaReceiver({
    connection,
    wallet: provider.wallet,
  });
  const SOL_PRICE_FEED_ID =
    '0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a';

  const solUsdPriceFeedAccount = pythSolanaReceiver
    .getPriceFeedAccountAddress(0, SOL_PRICE_FEED_ID)
    .toBase58();

  const solUsdPriceFeedAccountPubkey = new PublicKey(solUsdPriceFeedAccount);
  //从devnet拿到信息
  const feedAccountInfo = await devnetConnection.getAccountInfo(
    solUsdPriceFeedAccountPubkey
  );

  context.setAccount(solUsdPriceFeedAccountPubkey, feedAccountInfo);

  console.log('pricefeed:', solUsdPriceFeedAccount);

  console.log('Pyth Account Info:', accountInfo);

  program = new Program<Lending>(IDL as Lending, provider);

  banksClient = context.banksClient;

  signer = provider.wallet.payer;

  const mintUSDC = await createMint(
    // @ts-ignore
    banksClient,
    signer,
    signer.publicKey,
    null,
    2
  );

  const mintSOL = await createMint(
    // @ts-ignore
    banksClient,
    signer,
    signer.publicKey,
    null,
    2
  );

  [usdcBankAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from('treasury'), mintUSDC.toBuffer()],
    program.programId
  );

  [solBankAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from('treasury'), mintSOL.toBuffer()],
    program.programId
  );

  [solTokenAccount] = PublicKey.findProgramAddressSync(
    [Buffer.from('treasury'), mintSOL.toBuffer()],
    program.programId
  );

  console.log('USDC Bank Account', usdcBankAccount.toBase58());

  console.log('SOL Bank Account', solBankAccount.toBase58());
  it('Test Init User', async () => {
    const initUserTx = await program.methods
      .initUser(mintUSDC)
      .accounts({
        signer: signer.publicKey,
      })
      .rpc({ commitment: 'confirmed' });

    console.log('Create User Account', initUserTx);
  });

  it('Test Init and Fund USDC Bank', async () => {
    const initUSDCBankTx = await program.methods
      .initBank(new BN(1), new BN(1))
      .accounts({
        signer: signer.publicKey,
        mint: mintUSDC,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });

    console.log('Create USDC Bank Account', initUSDCBankTx);

    const amount = 10_000 * 10 ** 9;
    //给usdcbank账户转账
    const mintTx = await mintTo(
      // @ts-ignores
      banksClient,
      signer,
      mintUSDC,
      usdcBankAccount,
      signer,
      amount
    );

    try {
      const balance = await getTokenAccountBalance(connection, usdcBankAccount);
      console.log('====== usdcBankAccount Token Account Balance:', balance.toString());
    } catch (error) {
      console.error(error);
    }
  
  
    console.log('Mint to USDC Bank Signature:', mintTx);
  });

  it('Test Init amd Fund SOL Bank', async () => {
    const initSOLBankTx = await program.methods
      .initBank(new BN(1), new BN(1))
      .accounts({
        signer: signer.publicKey,
        mint: mintSOL,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });

    console.log('Create SOL Bank Account', initSOLBankTx);

    const amount = 10_000 * 10 ** 9;
    const mintSOLTx = await mintTo(
      // @ts-ignores
      banksClient,
      signer,
      mintSOL,
      solBankAccount,
      signer,
      amount
    );
    console.log('Mint to SOL Bank Signature:', mintSOLTx);
  });

  it('Create and Fund Token Account', async () => {
    UserUSDCTokenAccount = await createAccount(
      // @ts-ignores
      banksClient,
      signer,
      mintUSDC,
      signer.publicKey
    );

    console.log('USDC Token Account Created:', UserUSDCTokenAccount);

    const amount = new BN(10_000);
    const mintUSDCTx = await mintTo(
      // @ts-ignores
      banksClient,
      signer,
      mintUSDC,
      UserUSDCTokenAccount,
      signer,
      amount
    );

    
    
    
    console.log('Mint to USDC Bank Signature:', mintUSDCTx);
  });

  it('Test Deposit', async () => {
    
    try {
      const balance = await getTokenAccountBalance(connection, UserUSDCTokenAccount);
      console.log('====== UserUSDCTokenAccount Token Account Balance:', balance.toString());
    } catch (error) {
      console.error(error);
    }
    const depositUSDC = await program.methods
      .deposit(new BN(10_000))
      .accounts({
        signer: signer.publicKey,
        mint: mintUSDC,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });

      try {
        const balance = await getTokenAccountBalance(connection, UserUSDCTokenAccount);
        console.log('====== UserUSDCTokenAccount Token Account Balance:', balance.toString());
      } catch (error) {
        console.error(error);
      }
    console.log('Deposit USDC', depositUSDC);
  });

  it('Test Borrow', async () => {
    const borrowSOL = await program.methods
      .borrow(new BN(1))
      .accounts({
        signer: signer.publicKey,
        mint: mintSOL,
        tokenProgram: TOKEN_PROGRAM_ID,
        priceUpdate: solUsdPriceFeedAccount,
      })
      .rpc({ commitment: 'confirmed' });

    console.log('Borrow SOL', borrowSOL);
  });

  it('Test Repay', async () => {
    const repaySOL = await program.methods
      .repay(new BN(1))
      .accounts({
        signer: signer.publicKey,
        mint: mintSOL,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });

    console.log('Repay SOL', repaySOL);
  });

  it('Test Withdraw', async () => {
    const withdrawUSDC = await program.methods
      .withdraw(new BN(100))
      .accounts({
        signer: signer.publicKey,
        mint: mintUSDC,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc({ commitment: 'confirmed' });

    console.log('Withdraw USDC', withdrawUSDC);
  });

});


/**
 * 获取指定 SPL Token 账户的余额
 * @param connection Solana 连接对象
 * @param tokenAccount SPL Token 账户地址
 * @returns 余额（bigint），如果账户不存在则抛出错误
 */
export async function getTokenAccountBalance(
  connection: Connection,
  tokenAccount: PublicKey
): Promise<bigint> {
  // 获取账户信息
  const accountInfo = await connection.getAccountInfo(tokenAccount);
  if (!accountInfo) {
    throw new Error("Token account not found or no balance available.");
  }
  // 将账户数据转为 Buffer 对象
  const data = Buffer.from(accountInfo.data);
  // 假设 SPL Token 账户余额字段从偏移量 64 开始，占 8 个字节，采用小端格式读取 u64 数值
  const balance = data.readBigUInt64LE(64);
  return balance;
}