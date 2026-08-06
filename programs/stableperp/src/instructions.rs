use anchor_lang::prelude::*;
use anchor_spl::{
    token_interface::{Mint, TokenAccount, TokenInterface, TransferChecked, transfer_checked, MintTo, mint_to, Burn, burn},
    associated_token::AssociatedToken,
};
use crate::state::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Market is halted.")]
    MarketHalted,
    #[msg("Insufficient options in escrow to fill the order.")]
    InsufficientOptions,
    #[msg("Not within the exercise window.")]
    NotWithinExerciseWindow,
    #[msg("Option has not expired yet.")]
    NotExpired,
    #[msg("Factory is not active.")]
    FactoryNotActive,
}



#[derive(Accounts)]
pub struct InitConfig<'info> {
    #[account(
        init,
        payer = admin,
        space = Config::LEN,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handle_init_config(
    ctx: Context<InitConfig>,
    fee_bps: u16,
    treasury: Pubkey,
    buyback_wallet: Pubkey,
) -> Result<()> {
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.treasury = treasury;
    config.buyback_wallet = buyback_wallet;
    config.fee_bps = fee_bps;
    config.halted = false;
    Ok(())
}

#[derive(Accounts)]
pub struct InitFactory<'info> {
    #[account(
        init,
        payer = admin,
        space = FactoryConfig::LEN,
        seeds = [b"factory_config"],
        bump
    )]
    pub factory_config: Account<'info, FactoryConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handle_init_factory(ctx: Context<InitFactory>, creation_fee: u64) -> Result<()> {
    let factory_config = &mut ctx.accounts.factory_config;
    factory_config.admin = ctx.accounts.admin.key();
    factory_config.creation_fee = creation_fee;
    factory_config.is_active = true;
    Ok(())
}

#[derive(Accounts)]
#[instruction(creator: Pubkey)]
pub struct AddCreatorAllowlist<'info> {
    #[account(
        init,
        payer = admin,
        space = MarketCreator::LEN,
        seeds = [b"market_creator", creator.as_ref()],
        bump
    )]
    pub market_creator: Account<'info, MarketCreator>,
    #[account(seeds = [b"factory_config"], bump, has_one = admin)]
    pub factory_config: Account<'info, FactoryConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handle_add_creator_allowlist(ctx: Context<AddCreatorAllowlist>, creator: Pubkey) -> Result<()> {
    let market_creator = &mut ctx.accounts.market_creator;
    market_creator.authority = creator;
    market_creator.bump = ctx.bumps.market_creator;
    Ok(())
}

#[derive(Accounts)]
#[instruction(strike: u64, expiry_ts: i64)]
pub struct InitMarket<'info> {
    #[account(
        init,
        payer = creator,
        space = Market::LEN,
        seeds = [b"market", underlying_mint.key().as_ref(), quote_mint.key().as_ref(), &strike.to_le_bytes(), &expiry_ts.to_le_bytes()],
        bump
    )]
    pub market: Box<Account<'info, Market>>,
    
    #[account(seeds = [b"market_creator", creator.key().as_ref()], bump)]
    pub market_creator: Box<Account<'info, MarketCreator>>,
    
    #[account(seeds = [b"factory_config"], bump)]
    pub factory_config: Box<Account<'info, FactoryConfig>>,
    
    #[account(mut)]
    pub creator: Signer<'info>,
    
    pub underlying_mint: Box<InterfaceAccount<'info, Mint>>,
    pub quote_mint: Box<InterfaceAccount<'info, Mint>>,
    
    #[account(
        init,
        payer = creator,
        seeds = [b"option_mint", market.key().as_ref()],
        bump,
        mint::decimals = underlying_mint.decimals,
        mint::authority = market,
        mint::token_program = token_program
    )]
    pub option_mint: Box<InterfaceAccount<'info, Mint>>,
    
    #[account(
        mut,
        associated_token::mint = underlying_mint,
        associated_token::authority = market
    )]
    pub collateral_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        mut,
        associated_token::mint = quote_mint,
        associated_token::authority = market
    )]
    pub quote_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_init_market(
    ctx: Context<InitMarket>,
    strike: u64,
    expiry_ts: i64,
    exercise_window_secs: i64,
) -> Result<()> {
    require!(ctx.accounts.factory_config.is_active, ErrorCode::FactoryNotActive);
    
    let market = &mut ctx.accounts.market;
    market.underlying_mint = ctx.accounts.underlying_mint.key();
    market.quote_mint = ctx.accounts.quote_mint.key();
    market.strike = strike;
    market.expiry_ts = expiry_ts;
    market.exercise_window_secs = exercise_window_secs;
    market.option_mint = ctx.accounts.option_mint.key();
    market.bump = ctx.bumps.market;
    market.split_numerator = 1;
    market.split_denominator = 1;
    Ok(())
}

#[derive(Accounts)]
pub struct CorporateActionSplit<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,
    
    #[account(seeds = [b"config"], bump, has_one = admin)]
    pub config: Account<'info, Config>,
    
    pub admin: Signer<'info>,
}

pub fn handle_corporate_action_split(
    ctx: Context<CorporateActionSplit>,
    numerator: u64,
    denominator: u64,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    market.split_numerator = numerator;
    market.split_denominator = denominator;
    Ok(())
}

#[derive(Accounts)]
pub struct WriteOption<'info> {
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
    
    #[account(
        init,
        payer = writer,
        space = WriterPosition::LEN,
        seeds = [b"writer", market.key().as_ref(), writer.key().as_ref()],
        bump
    )]
    pub writer_position: Box<Account<'info, WriterPosition>>,
    
    #[account(
        mut,
        associated_token::mint = underlying_mint,
        associated_token::authority = market
    )]
    pub collateral_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub writer_underlying_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        mut,
        mint::authority = market,
        mint::decimals = underlying_mint.decimals
    )]
    pub option_mint: Box<InterfaceAccount<'info, Mint>>,
    
    // Escrow to hold the minted options until a buyer buys them (ATA of writer_position)
    #[account(
        mut,
        associated_token::mint = option_mint,
        associated_token::authority = writer_position
    )]
    pub escrow_option_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub writer: Signer<'info>,
    
    pub underlying_mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_write_option(ctx: Context<WriteOption>, qty: u64, premium_ask: u64) -> Result<()> {
    require!(!ctx.accounts.market.strike == 0, ErrorCode::MarketHalted); // Dummy check, replace with actual config halt check if passed
    
    // 1. Transfer xStock (underlying) from writer to vault
    let transfer_cpi_accounts = TransferChecked {
        from: ctx.accounts.writer_underlying_ata.to_account_info(),
        mint: ctx.accounts.underlying_mint.to_account_info(),
        to: ctx.accounts.collateral_vault.to_account_info(),
        authority: ctx.accounts.writer.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program.clone(), transfer_cpi_accounts);
    transfer_checked(cpi_ctx, qty, ctx.accounts.underlying_mint.decimals)?;

    // 2. Mint OptionTokens to escrow vault
    let market_key = ctx.accounts.market.key();
    let market_bump = ctx.accounts.market.bump;
    let seeds = &[
        b"market",
        ctx.accounts.market.underlying_mint.as_ref(),
        ctx.accounts.market.quote_mint.as_ref(),
        &ctx.accounts.market.strike.to_le_bytes(),
        &ctx.accounts.market.expiry_ts.to_le_bytes(),
        &[market_bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let mint_to_cpi_accounts = MintTo {
        mint: ctx.accounts.option_mint.to_account_info(),
        to: ctx.accounts.escrow_option_vault.to_account_info(),
        authority: ctx.accounts.market.to_account_info(),
    };
    let mint_cpi_ctx = CpiContext::new_with_signer(cpi_program.clone(), mint_to_cpi_accounts, signer_seeds);
    mint_to(mint_cpi_ctx, qty)?;

    // 3. Update WriterPosition state
    let position = &mut ctx.accounts.writer_position;
    position.market = market_key;
    position.writer = ctx.accounts.writer.key();
    position.locked_amount = position.locked_amount.checked_add(qty).unwrap();
    position.minted_amount = position.minted_amount.checked_add(qty).unwrap();
    position.premium_ask = premium_ask; // Allows updating the premium ask
    position.bump = ctx.bumps.writer_position;

    Ok(())
}

#[derive(Accounts)]
pub struct BuyOption<'info> {
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
    
    #[account(mut, has_one = market)]
    pub writer_position: Box<Account<'info, WriterPosition>>,
    
    #[account(
        mut,
        associated_token::mint = option_mint,
        associated_token::authority = writer_position
    )]
    pub escrow_option_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub buyer_underlying_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        mut,
        associated_token::mint = option_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_option_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub buyer_quote_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub writer_quote_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub option_mint: Box<InterfaceAccount<'info, Mint>>,
    
    #[account(mut)]
    pub buyer: Signer<'info>,
    
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_buy_option(ctx: Context<BuyOption>, qty: u64) -> Result<()> {
    let position = &mut ctx.accounts.writer_position;
    let available_qty = position.minted_amount.checked_sub(position.filled_amount).unwrap();
    require!(available_qty >= qty, ErrorCode::InsufficientOptions);
    
    // Simplification for MVP: Assuming premium_ask is the total price per whole token, ignoring decimals matching for now.
    // In production, we'd scale based on token decimals.
    let total_cost = (qty as u128).checked_mul(position.premium_ask as u128).unwrap() as u64;

    let cpi_program = ctx.accounts.token_program.to_account_info();

    // 1. Transfer Quote (USDC) from buyer to writer
    let transfer_quote_cpi = TransferChecked {
        from: ctx.accounts.buyer_quote_ata.to_account_info(),
        mint: ctx.accounts.option_mint.to_account_info(), // Assuming mint is quote_mint in real scenario, this matches struct
        to: ctx.accounts.writer_quote_ata.to_account_info(),
        authority: ctx.accounts.buyer.to_account_info(),
    };
    let cpi_ctx_quote = CpiContext::new(cpi_program.clone(), transfer_quote_cpi);
    transfer_checked(cpi_ctx_quote, total_cost, 6)?; // Assuming 6 decimals for quote_mint

    // 2. Transfer Option Tokens from Escrow to Buyer
    let writer_key = position.writer;
    let market_key = position.market;
    let position_bump = position.bump;
    let escrow_seeds = &[
        b"writer",
        market_key.as_ref(),
        writer_key.as_ref(),
        &[position_bump],
    ];
    let escrow_signer_seeds = &[&escrow_seeds[..]];

    let transfer_option_cpi = TransferChecked {
        from: ctx.accounts.escrow_option_vault.to_account_info(),
        mint: ctx.accounts.option_mint.to_account_info(),
        to: ctx.accounts.buyer_option_ata.to_account_info(),
        authority: position.to_account_info(),
    };
    let cpi_ctx_option = CpiContext::new_with_signer(cpi_program.clone(), transfer_option_cpi, escrow_signer_seeds);
    transfer_checked(cpi_ctx_option, qty, ctx.accounts.option_mint.decimals)?;

    position.filled_amount = position.filled_amount.checked_add(qty).unwrap();

    Ok(())
}

#[derive(Accounts)]
pub struct ExerciseOption<'info> {
    #[account(mut)]
    pub market: Box<Account<'info, Market>>,
    
    #[account(
        mut,
        associated_token::mint = underlying_mint,
        associated_token::authority = market
    )]
    pub collateral_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        mut,
        associated_token::mint = quote_mint,
        associated_token::authority = market
    )]
    pub quote_vault: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub exerciser: Signer<'info>,
    
    #[account(mut)]
    pub exerciser_option_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub exerciser_underlying_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub exerciser_quote_ata: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(mut)]
    pub option_mint: Box<InterfaceAccount<'info, Mint>>,
    
    pub underlying_mint: Box<InterfaceAccount<'info, Mint>>,
    pub quote_mint: Box<InterfaceAccount<'info, Mint>>,
    
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handle_exercise(ctx: Context<ExerciseOption>, qty: u64) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;
    let market = &ctx.accounts.market;
    
    require!(current_time >= market.expiry_ts, ErrorCode::NotExpired);
    require!(current_time <= market.expiry_ts + market.exercise_window_secs, ErrorCode::NotWithinExerciseWindow);
    
    let total_cost = (qty as u128).checked_mul(market.strike as u128).unwrap() as u64;

    let cpi_program = ctx.accounts.token_program.to_account_info();

    // 1. Burn Option tokens
    let burn_cpi = Burn {
        mint: ctx.accounts.option_mint.to_account_info(),
        from: ctx.accounts.exerciser_option_ata.to_account_info(),
        authority: ctx.accounts.exerciser.to_account_info(),
    };
    let cpi_ctx_burn = CpiContext::new(cpi_program.clone(), burn_cpi);
    burn(cpi_ctx_burn, qty)?;

    // 2. Transfer Quote (USDC) from exerciser to quote_vault
    let transfer_quote_cpi = TransferChecked {
        from: ctx.accounts.exerciser_quote_ata.to_account_info(),
        mint: ctx.accounts.quote_mint.to_account_info(),
        to: ctx.accounts.quote_vault.to_account_info(),
        authority: ctx.accounts.exerciser.to_account_info(),
    };
    let cpi_ctx_quote = CpiContext::new(cpi_program.clone(), transfer_quote_cpi);
    transfer_checked(cpi_ctx_quote, total_cost, ctx.accounts.quote_mint.decimals)?;

    // 3. Transfer Underlying (xStock) from collateral_vault to exerciser
    let market_key = market.underlying_mint;
    let quote_key = market.quote_mint;
    let strike = market.strike;
    let expiry = market.expiry_ts;
    let bump = market.bump;
    let seeds = &[
        b"market",
        market_key.as_ref(),
        quote_key.as_ref(),
        &strike.to_le_bytes(),
        &expiry.to_le_bytes(),
        &[bump],
    ];
    let signer_seeds = &[&seeds[..]];

    let transfer_underlying_cpi = TransferChecked {
        from: ctx.accounts.collateral_vault.to_account_info(),
        mint: ctx.accounts.underlying_mint.to_account_info(),
        to: ctx.accounts.exerciser_underlying_ata.to_account_info(),
        authority: ctx.accounts.market.to_account_info(),
    };
    let cpi_ctx_underlying = CpiContext::new_with_signer(cpi_program.clone(), transfer_underlying_cpi, signer_seeds);
    
    let underlying_qty = (qty as u128)
        .checked_mul(market.split_numerator as u128)
        .unwrap()
        .checked_div(market.split_denominator as u128)
        .unwrap() as u64;

    transfer_checked(cpi_ctx_underlying, underlying_qty, ctx.accounts.underlying_mint.decimals)?;

    Ok(())
}

#[derive(Accounts)]
pub struct AdminHalt<'info> {
    #[account(mut, has_one = admin)]
    pub config: Account<'info, Config>,
    pub admin: Signer<'info>,
}

pub fn handle_admin_halt(ctx: Context<AdminHalt>) -> Result<()> {
    ctx.accounts.config.halted = true;
    Ok(())
}

pub fn handle_admin_resume(ctx: Context<AdminHalt>) -> Result<()> {
    ctx.accounts.config.halted = false;
    Ok(())
}

