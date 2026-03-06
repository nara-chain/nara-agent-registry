use anchor_lang::prelude::*;

// declare_id!("AgentRegistry111111111111111111111111111111");
declare_id!("8VNuYRUPWyTx2tuKX1Mxq7TZHuA5gbT3LpgGUe9XC3iY");

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

declare_program!(nara_quest);

use instructions::*;

#[program]
pub mod nara_agent_registry {
    use super::*;

    pub fn init_config(ctx: Context<InitConfig>) -> Result<()> {
        instructions::init_config::init_config(ctx)
    }

    pub fn update_admin(ctx: Context<UpdateAdmin>, new_admin: Pubkey) -> Result<()> {
        instructions::update_admin::update_admin(ctx, new_admin)
    }

    pub fn update_fee_recipient(ctx: Context<UpdateFeeRecipient>, new_recipient: Pubkey) -> Result<()> {
        instructions::update_fee_recipient::update_fee_recipient(ctx, new_recipient)
    }

    pub fn update_register_fee(ctx: Context<UpdateRegisterFee>, new_fee: u64) -> Result<()> {
        instructions::update_register_fee::update_register_fee(ctx, new_fee)
    }

    pub fn update_points_config(ctx: Context<UpdatePointsConfig>, points_self: u64, points_referral: u64) -> Result<()> {
        instructions::update_points_config::update_points_config(ctx, points_self, points_referral)
    }

    pub fn register_agent(ctx: Context<RegisterAgent>, agent_id: String) -> Result<()> {
        instructions::register_agent::register_agent(ctx, agent_id)
    }

    pub fn transfer_authority(
        ctx: Context<TransferAuthority>,
        agent_id: String,
        new_authority: Pubkey,
    ) -> Result<()> {
        instructions::transfer_authority::transfer_authority(ctx, agent_id, new_authority)
    }

    pub fn set_bio(
        ctx: Context<SetBio>,
        agent_id: String,
        bio: String,
    ) -> Result<()> {
        instructions::set_bio::set_bio(ctx, agent_id, bio)
    }

    pub fn set_metadata(ctx: Context<SetMetadata>, agent_id: String, data: String) -> Result<()> {
        instructions::set_metadata::set_metadata(ctx, agent_id, data)
    }

    pub fn init_buffer(ctx: Context<InitBuffer>, agent_id: String, total_len: u32) -> Result<()> {
        instructions::init_buffer::init_buffer(ctx, agent_id, total_len)
    }

    pub fn write_to_buffer(
        ctx: Context<WriteToBuffer>,
        agent_id: String,
        offset: u32,
        data: Vec<u8>,
    ) -> Result<()> {
        instructions::write_to_buffer::write_to_buffer(ctx, agent_id, offset, data)
    }

    pub fn finalize_memory_new(ctx: Context<FinalizeMemoryNew>, agent_id: String) -> Result<()> {
        instructions::finalize_memory_new::finalize_memory_new(ctx, agent_id)
    }

    pub fn finalize_memory_update(ctx: Context<FinalizeMemoryUpdate>, agent_id: String) -> Result<()> {
        instructions::finalize_memory_update::finalize_memory_update(ctx, agent_id)
    }

    pub fn finalize_memory_append(ctx: Context<FinalizeMemoryAppend>, agent_id: String) -> Result<()> {
        instructions::finalize_memory_append::finalize_memory_append(ctx, agent_id)
    }

    pub fn close_buffer(ctx: Context<CloseBuffer>, agent_id: String) -> Result<()> {
        instructions::close_buffer::close_buffer(ctx, agent_id)
    }

    pub fn delete_agent(ctx: Context<DeleteAgent>, agent_id: String) -> Result<()> {
        instructions::delete_agent::delete_agent(ctx, agent_id)
    }

    pub fn log_activity(
        ctx: Context<LogActivity>,
        agent_id: String,
        model: String,
        activity: String,
        log: String,
    ) -> Result<()> {
        instructions::log_activity::log_activity(ctx, agent_id, model, activity, log)
    }
}
