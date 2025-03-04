use anchor_lang::prelude::*;
use std::f32::consts::E;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface,TransferChecked,transfer_checked};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use crate::constants::*;
use crate::state::*;
use crate::error::ErrorCode;

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct Borrow<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    pub mint:InterfaceAccount<'info,Mint>,

    #[account(
        mut,
        seeds = [mint.key().as_ref()],
        bump
    )]
    pub bank: Account<'info,Bank>,

    #[account(
        mut,
        seeds = [b"treasury", mint.key().as_ref()],
        bump
    )]
    pub bank_token_account: InterfaceAccount<'info,TokenAccount>,

    #[account(
        mut,
        seeds = [signer.key().as_ref()],
        bump,
    )]
    pub user_account: Account<'info,User>,

    #[account(
        init_if_needed,//不知道是否借出的代币借用者是否本来拥有
        payer = signer,
        associated_token::mint = mint, 
        associated_token::authority = signer,
        associated_token::token_program = token_program,

    )]
    pub user_token_account: InterfaceAccount<'info,TokenAccount>,
    pub price_update: Account<'info, PriceUpdateV2>,
    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

pub fn process_borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
    let bank = &mut ctx.accounts.bank;
    let user = &mut ctx.accounts.user_account;
    let price_update = &ctx.accounts.price_update;
    let total_collateral: u64;


    // 通过调用 Pyth 预言机获取当前价格，计算出以美元计价的总抵押品价值。
    // 如果存款是 SOL，还会计算存款的累计利息，再乘以 SOL/USD 价格；
    // 如果是 USDC，则直接乘以 USDC/USD 价格。
    match ctx.accounts.mint.to_account_info().key() {
        key if key == user.usdc_address => {  // 检查用户存款的代币类型是否为 USDC
            let sol_feed_id = get_feed_id_from_hex(SOL_USD_FEED_ID)?; // 获取 SOL/USD 预言机的 feed ID
            let sol_price = price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &sol_feed_id)?; 
            // 获取最新的 SOL/USD 价格，确保价格数据没有超过 MAXIMUM_AGE 的时间限制
            
            let accrued_interest = calculate_accrued_interest(user.deposited_sol, bank.interest_rate, user.last_updated_borrowed)?;
            // 计算用户存款在上次更新时间后的累计利息，基于用户存款金额、银行利率和上次更新时间
            
            total_collateral = sol_price.price as u64 * (user.deposited_sol + accrued_interest);
            // 将用户存款金额和累计利息相加，乘以当前 SOL/USD 价格，得出以美元计价的总抵押品价值
        },
        _ => {  // 如果存款不是 USDC，假设存款类型是 SOL
            let usdc_feed_id = get_feed_id_from_hex(USDC_USD_FEED_ID)?; // 获取 USDC/USD 预言机的 feed ID
            let usdc_price = price_update.get_price_no_older_than(&Clock::get()?, MAXIMUM_AGE, &usdc_feed_id)?;
            // 获取最新的 USDC/USD 价格，确保价格数据没有超出时效
            
            total_collateral = usdc_price.price as u64 * user.deposited_usdc;
            // 直接将用户的 USDC 存款数量乘以 USDC/USD 价格，得出以美元计价的总抵押品价值
        }
    }   

    let borrowable_amount = total_collateral as u64 *  bank.liquidation_threshold;
    if borrowable_amount < amount {
        return Err(ErrorCode::OverBorrowableAmount.into());
    }       

    let transfer_cpi_accounts = TransferChecked {
        from: ctx.accounts.bank_token_account.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.bank_token_account.to_account_info(),
    };

    let cpi_program = ctx.accounts.token_program.to_account_info();
    let mint_key = ctx.accounts.mint.key();
    let signer_seeds: &[&[&[u8]]] = &[
        &[
            b"treasury",
            mint_key.as_ref(),
            &[ctx.bumps.bank_token_account],
        ],
    ];
    let cpi_ctx = CpiContext::new(cpi_program, transfer_cpi_accounts).with_signer(signer_seeds);
    let decimals = ctx.accounts.mint.decimals;

    transfer_checked(cpi_ctx, amount, decimals)?;


    if bank.total_borrowed == 0 {
        bank.total_borrowed = amount;
        bank.total_borrowed_shares = amount;
    } 

    
    let users_shares = amount.checked_mul(bank.total_borrowed_shares)
    .unwrap()
    .checked_div(bank.total_borrowed)
    .unwrap();

    bank.total_borrowed += amount;
    bank.total_borrowed_shares += users_shares; 

    match ctx.accounts.mint.to_account_info().key() {
        key if key == user.usdc_address => {
            user.borrowed_usdc += amount;
            user.borrowed_usdc_shares += users_shares;  // 更新借款份额，而非存款份额
        },
        _ => {
            user.borrowed_sol += amount;
            user.borrowed_sol_shares += users_shares;  // 同样更新借款份额
        }
    }

    user.last_updated_borrowed = Clock::get()?.unix_timestamp;
    Ok(())
    
}
// 该函数用于计算存款金额在一段时间内的累计利息，并返回存款的新值（包括本金和利息）。
// 假设利率是连续复利（compound interest），通过自然指数函数 e^x 计算利息增长。
fn calculate_accrued_interest(deposited: u64, interest_rate: u64, last_update: i64) -> Result<u64> {
    let current_time = Clock::get()?.unix_timestamp;
    let time_elapsed = current_time - last_update;
    // 使用连续复利公式计算新的存款值：
    // A = P * e^(r * t)
    // 其中：
    // - P: 初始存款金额 (deposited)
    // - r: 利率 (interest_rate)
    // - t: 时间 (time_elapsed)
    let new_value = (deposited as f64 * E.powf(interest_rate as f32 * time_elapsed as f32) as f64) as u64;
    Ok(new_value)
}