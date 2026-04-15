use anchor_lang::prelude::*;

// declare_id!("AgentRegistry111111111111111111111111111111");
declare_id!("8VNuYRUPWyTx2tuKX1Mxq7TZHuA5gbT3LpgGUe9XC3iY");

pub mod constants;
pub mod error;
pub mod instructions;
pub mod seeds;
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

    pub fn update_register_fee(ctx: Context<UpdateRegisterFee>, fee: u64, fee_7: u64, fee_6: u64, fee_5: u64) -> Result<()> {
        instructions::update_register_fee::update_register_fee(ctx, fee, fee_7, fee_6, fee_5)
    }

    pub fn update_points_config(ctx: Context<UpdatePointsConfig>, points_self: u64, points_referral: u64) -> Result<()> {
        instructions::update_points_config::update_points_config(ctx, points_self, points_referral)
    }

    pub fn update_activity_config(ctx: Context<UpdateActivityConfig>, activity_reward: u64, referral_activity_reward: u64) -> Result<()> {
        instructions::update_activity_config::update_activity_config(ctx, activity_reward, referral_activity_reward)
    }

    pub fn update_referral_config(
        ctx: Context<UpdateReferralConfig>,
        referral_discount_bps: u64,
        referral_share_bps: u64,
        referral_register_points: u64,
    ) -> Result<()> {
        instructions::update_referral_config::update_referral_config(ctx, referral_discount_bps, referral_share_bps, referral_register_points)
    }

    pub fn register_agent(ctx: Context<RegisterAgent>, agent_id: String) -> Result<()> {
        instructions::register_agent::register_agent(ctx, agent_id)
    }

    pub fn register_agent_with_referral(ctx: Context<RegisterAgentWithReferral>, agent_id: String) -> Result<()> {
        instructions::register_agent::register_agent_with_referral(ctx, agent_id)
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

    pub fn set_referral(ctx: Context<SetReferral>, agent_id: String) -> Result<()> {
        instructions::set_referral::set_referral(ctx, agent_id)
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

    pub fn log_activity_with_referral(
        ctx: Context<LogActivityWithReferral>,
        agent_id: String,
        model: String,
        activity: String,
        log: String,
    ) -> Result<()> {
        instructions::log_activity::log_activity_with_referral(ctx, agent_id, model, activity, log)
    }

    pub fn withdraw_fees(ctx: Context<WithdrawFees>, amount: u64) -> Result<()> {
        instructions::withdraw_fees::withdraw_fees(ctx, amount)
    }

    pub fn expand_config(ctx: Context<ExpandConfig>, extend_size: u64) -> Result<()> {
        instructions::expand_config::expand_config(ctx, extend_size)
    }

    pub fn set_twitter(ctx: Context<SetTwitter>, agent_id: String, username: String, tweet_url: String) -> Result<()> {
        instructions::set_twitter::set_twitter(ctx, agent_id, username, tweet_url)
    }

    pub fn verify_twitter(ctx: Context<VerifyTwitter>, agent_id: String, username: String) -> Result<()> {
        instructions::verify_twitter::verify_twitter(ctx, agent_id, username)
    }

    pub fn approve_rejected_twitter(ctx: Context<ApproveRejectedTwitter>, agent_id: String, username: String) -> Result<()> {
        instructions::approve_rejected_twitter::approve_rejected_twitter(ctx, agent_id, username)
    }

    pub fn reject_twitter(ctx: Context<RejectTwitter>, agent_id: String) -> Result<()> {
        instructions::reject_twitter::reject_twitter(ctx, agent_id)
    }

    pub fn reject_twitter_with_reason(ctx: Context<RejectTwitterWithReason>, agent_id: String, reason: u64) -> Result<()> {
        instructions::reject_twitter_with_reason::reject_twitter_with_reason(ctx, agent_id, reason)
    }

    pub fn unbind_twitter(ctx: Context<UnbindTwitter>, agent_id: String, username: String) -> Result<()> {
        instructions::unbind_twitter::unbind_twitter(ctx, agent_id, username)
    }

    pub fn update_twitter_verifier(ctx: Context<UpdateTwitterVerifier>, new_verifier: Pubkey) -> Result<()> {
        instructions::update_twitter_verifier::update_twitter_verifier(ctx, new_verifier)
    }

    pub fn update_twitter_verification_config(
        ctx: Context<UpdateTwitterVerificationConfig>,
        fee: u64,
        reward: u64,
        points: u64,
    ) -> Result<()> {
        instructions::update_twitter_verification_config::update_twitter_verification_config(ctx, fee, reward, points)
    }

    pub fn withdraw_twitter_verify_fees(ctx: Context<WithdrawTwitterVerifyFees>, amount: u64) -> Result<()> {
        instructions::withdraw_twitter_verify_fees::withdraw_twitter_verify_fees(ctx, amount)
    }

    pub fn submit_tweet(ctx: Context<SubmitTweet>, agent_id: String, tweet_id: u128) -> Result<()> {
        instructions::submit_tweet::submit_tweet(ctx, agent_id, tweet_id)
    }

    pub fn approve_tweet(ctx: Context<ApproveTweet>, agent_id: String, tweet_id: u128) -> Result<()> {
        instructions::approve_tweet::approve_tweet(ctx, agent_id, tweet_id)
    }

    pub fn reject_tweet(ctx: Context<RejectTweet>, agent_id: String) -> Result<()> {
        instructions::reject_tweet::reject_tweet(ctx, agent_id)
    }

    pub fn update_tweet_verify_config(
        ctx: Context<UpdateTweetVerifyConfig>,
        reward: u64,
        points: u64,
    ) -> Result<()> {
        instructions::update_tweet_verify_config::update_tweet_verify_config(ctx, reward, points)
    }
}
