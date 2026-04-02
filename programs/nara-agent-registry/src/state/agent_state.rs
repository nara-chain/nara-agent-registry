use anchor_lang::prelude::*;

/// PDA metadata account for an agent, seeds = [SEED_AGENT, agent_id.as_bytes()].
#[account(zero_copy)]
#[repr(C)]
pub struct AgentState {
    pub authority: Pubkey,
    pub pending_buffer: Pubkey,   // Pubkey::default() = no pending buffer
    pub memory: Pubkey,           // Pubkey::default() = no memory yet
    pub created_at: i64,
    pub updated_at: i64,
    pub version: u32,
    pub agent_id_len: u32,
    pub agent_id: [u8; 32],
    pub referral_id_len: u32,
    pub referral_id: [u8; 32],
    pub referral_count: u32,
    pub _reserved: [u8; 64],
}

impl AgentState {
    pub fn agent_id_str(&self) -> &str {
        std::str::from_utf8(&self.agent_id[..self.agent_id_len as usize]).unwrap_or("")
    }

    pub fn referral_id_str(&self) -> &str {
        std::str::from_utf8(&self.referral_id[..self.referral_id_len as usize]).unwrap_or("")
    }

    pub fn has_referral(&self) -> bool {
        self.referral_id_len > 0
    }
}
