use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Bank {
    /// 权限，控制可以对 Bank 状态进行更改的账户
    pub authority: Pubkey,
    /// 资产的 Mint 地址
    pub mint_address: Pubkey,
    /// 银行中当前的代币总量（存款总量）
    pub total_deposits: u64,
    /// 银行中当前的存款份额总数
    pub total_deposit_shares: u64,
    /// 银行中当前借出的代币总量（借款总量）
    pub total_borrowed: u64,
    /// 银行中当前借款份额总数
    pub total_borrowed_shares: u64,
    /// 当贷款的抵押价值比率达到此值时，贷款被视为未充分抵押，可被清算（Liquidation Threshold）
    pub liquidation_threshold: u64,
    /// 清算奖励比例（清算时清算人可以额外获得的抵押品百分比）
    pub liquidation_bonus: u64,
    /// 清算时可以清算的最大抵押品百分比（Liquidation Close Factor）
    pub liquidation_close_factor: u64,
    /// 可借出的最大抵押比例（Loan-to-Value Ratio）
    pub max_ltv: u64,
    /// 上次更新的时间戳
    pub last_updated: i64,
    /// 当前利率
    pub interest_rate: u64,
}