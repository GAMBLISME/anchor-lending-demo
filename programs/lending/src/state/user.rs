use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct User {
    // 用户钱包的公钥
    pub owner: Pubkey,
    /// 用户在 SOL 银行存入的代币数量
    pub deposited_sol: u64,
    /// 用户在 SOL 银行的存款份额数量
    pub deposited_sol_shares: u64,
    /// 用户在 SOL 银行借出的代币数量
    pub borrowed_sol: u64,
    /// 用户在 SOL 银行的借款份额数量
    pub borrowed_sol_shares: u64,
    /// 用户在 USDC 银行存入的代币数量
    pub deposited_usdc: u64,
    /// 用户在 USDC 银行的存款份额数量
    pub deposited_usdc_shares: u64,
    /// 用户在 USDC 银行借出的代币数量
    pub borrowed_usdc: u64,
    /// 用户在 USDC 银行的借款份额数量
    pub borrowed_usdc_shares: u64,
    /// USDC 的 Mint 地址
    pub usdc_address: Pubkey,
    /// 用户的当前健康因子（Health Factor），衡量用户借贷安全性的指标
    pub health_factor: u64,

    pub last_updated_borrowed: i64,

    pub last_updated_deposited: i64,
}
