use anchor_lang::prelude::*;

pub mod state;
pub mod instructions;

use instructions::*;

declare_id!("D2pAeo437UfwD7jALV8rDB5fpHJJBt5GzDeDka8485mS");

#[program]
pub mod stableperp {
    use super::*;

    pub fn init_config(
        ctx: Context<InitConfig>,
        fee_bps: u16,
        treasury: Pubkey,
        buyback_wallet: Pubkey,
    ) -> Result<()> {
        handle_init_config(ctx, fee_bps, treasury, buyback_wallet)
    }

    pub fn init_factory(ctx: Context<InitFactory>, creation_fee: u64) -> Result<()> {
        handle_init_factory(ctx, creation_fee)
    }

    pub fn add_creator_allowlist(ctx: Context<AddCreatorAllowlist>, creator: Pubkey) -> Result<()> {
        handle_add_creator_allowlist(ctx, creator)
    }

    pub fn init_market(
        ctx: Context<InitMarket>,
        strike: u64,
        expiry_ts: i64,
        exercise_window_secs: i64,
        is_synthetic: bool,
        payout_cap: u64,
    ) -> Result<()> {
        handle_init_market(ctx, strike, expiry_ts, exercise_window_secs, is_synthetic, payout_cap)
    }

    pub fn write_option(
        ctx: Context<WriteOption>,
        qty: u64,
        premium_ask: u64,
    ) -> Result<()> {
        handle_write_option(ctx, qty, premium_ask)
    }

    pub fn buy_option(
        ctx: Context<BuyOption>,
        qty: u64,
    ) -> Result<()> {
        handle_buy_option(ctx, qty)
    }

    pub fn exercise_option(
        ctx: Context<ExerciseOption>,
        qty: u64,
    ) -> Result<()> {
        handle_exercise(ctx, qty)
    }

    pub fn admin_halt(ctx: Context<AdminHalt>) -> Result<()> {
        handle_admin_halt(ctx)
    }

    pub fn admin_resume(ctx: Context<AdminHalt>) -> Result<()> {
        handle_admin_resume(ctx)
    }

    pub fn corporate_action_split(
        ctx: Context<CorporateActionSplit>,
        numerator: u64,
        denominator: u64,
    ) -> Result<()> {
        handle_corporate_action_split(ctx, numerator, denominator)
    }

    pub fn reclaim_collateral(ctx: Context<ReclaimCollateral>) -> Result<()> {
        handle_reclaim_collateral(ctx)
    }
}
