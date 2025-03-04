use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    Mint, 
    TokenAccount,
    TokenInterface,
    TransferChecked,
    transfer_checked};
use crate::state::*;

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub mint: InterfaceAccount<'info,Mint>,
    #[account(
        mut,
        seeds = [mint.key().as_ref()],
        bump
    )]
    pub bank: Account<'info,Bank>,
    
    #[account(
        mut,
        seeds = [b"treasury", mint.key().as_ref()],
        bump, 
    )]
   pub bank_token_account: InterfaceAccount<'info,TokenAccount>,

   #[account(
        mut,
        seeds = [signer.key().as_ref()],
        bump,
    )]
   pub user_account: Account<'info,User>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = signer,
        associated_token::token_program = token_program
    )]
   pub user_token_account: InterfaceAccount<'info,TokenAccount>,
   pub token_program: Interface<'info, TokenInterface>,
   pub associated_token_program: Program<'info, AssociatedToken>,
   pub system_program: Program<'info,System>
}

// 1. 通过 CPI 从用户的代币账户向银行的代币账户转账
// 2. 计算需要添加到银行的新股份
// 3. 更新用户的存款金额和总抵押品价值
// 4. 更新银行的总存款和总存款股份
// 5. 更新用户的健康因子 ??
pub fn process_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let transfer_cpi_account =TransferChecked{
        from: ctx.accounts.user_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.bank_token_account.to_account_info(),
        authority: ctx.accounts.signer.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_context = CpiContext::new(cpi_program, transfer_cpi_account);
    let decimals = ctx.accounts.mint.decimals;
    transfer_checked(cpi_context, amount, decimals)?;
    // 计算需要添加到银行的新股份
    let bank = &mut ctx.accounts.bank;

    // 注意：Rust 中的 `checked_` 前缀用于在执行操作时进行安全检查，
    // 以防止可能发生的算术溢出或其他计算错误。
    // 如果出现此类错误，这些方法会返回 `None`，而不是引发程序崩溃（panic）。
    if bank.total_deposits == 0 {
        bank.total_deposits = amount;
        bank.total_deposit_shares = amount;
    }
  
    //根据用户存款的比例，计算用户可以获得的 存款份额（shares）。
    let users_shares = amount
    .checked_mul(bank.total_deposit_shares)  //计算用户存款金额在银行总存款中的比例
    .unwrap()
    .checked_div(bank.total_deposits)
    .unwrap();

    let user = &mut ctx.accounts.user_account;


    match ctx.accounts.mint.to_account_info().key() {
        key if key == user.usdc_address => {
            user.deposited_usdc += amount;
            user.deposited_usdc_shares += users_shares;
        },
        _ => {
            user.deposited_sol += amount;
            user.deposited_sol_shares += users_shares; 
        }
    }

    bank.total_deposits += amount;
    bank.total_deposit_shares += users_shares;

    user.last_updated_deposited = Clock::get()?.unix_timestamp;

    Ok(())
}