use anchor_lang::prelude::*;

#[account]
pub struct Config {
    pub admin: Pubkey,
    pub treasury: Pubkey,
    pub buyback_wallet: Pubkey,
    pub fee_bps: u16,
    pub halted: bool,
}

impl Config {
    // 8 (discriminator) + 32 + 32 + 32 + 2 + 1
    pub const LEN: usize = 8 + 32 + 32 + 32 + 2 + 1;
}

#[account]
pub struct Market {
    pub underlying_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub strike: u64,
    pub expiry_ts: i64,
    pub exercise_window_secs: i64,
    pub option_mint: Pubkey,
    pub bump: u8,
}

impl Market {
    // 8 (discriminator) + 32 + 32 + 8 + 8 + 8 + 32 + 1
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 8 + 32 + 1;
}

#[account]
pub struct WriterPosition {
    pub market: Pubkey,
    pub writer: Pubkey,
    pub locked_amount: u64,
    pub minted_amount: u64,
    pub premium_ask: u64,
    pub filled_amount: u64,
    pub bump: u8,
}

impl WriterPosition {
    // 8 (discriminator) + 32 + 32 + 8 + 8 + 8 + 8 + 1
    pub const LEN: usize = 8 + 32 + 32 + 8 + 8 + 8 + 8 + 1;
}

#[account]
pub struct FactoryConfig {
    pub admin: Pubkey,
    pub creation_fee: u64,
    pub is_active: bool,
}

impl FactoryConfig {
    // 8 (discriminator) + 32 + 8 + 1
    pub const LEN: usize = 8 + 32 + 8 + 1;
}

#[account]
pub struct MarketCreator {
    pub authority: Pubkey,
    pub bump: u8,
}

impl MarketCreator {
    // 8 (discriminator) + 32 + 1
    pub const LEN: usize = 8 + 32 + 1;
}
