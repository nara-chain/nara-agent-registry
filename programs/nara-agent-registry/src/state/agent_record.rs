use anchor_lang::prelude::*;

/// PDA metadata account for an agent, seeds = [b"agent", agent_id.as_bytes()].
#[account(zero_copy)]
#[repr(C)]
pub struct AgentRecord {
    pub authority: Pubkey,
    pub pending_buffer: Pubkey,   // Pubkey::default() = no pending buffer
    pub memory: Pubkey,           // Pubkey::default() = no memory yet
    pub created_at: i64,
    pub updated_at: i64,
    pub points: u64,
    pub version: u32,
    pub agent_id_len: u32,
    pub agent_id: [u8; 32],
    pub _reserved: [u8; 64],
}

impl AgentRecord {
    pub fn agent_id_str(&self) -> &str {
        std::str::from_utf8(&self.agent_id[..self.agent_id_len as usize]).unwrap_or("")
    }
}
