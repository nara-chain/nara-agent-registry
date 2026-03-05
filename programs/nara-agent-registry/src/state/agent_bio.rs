use anchor_lang::prelude::*;

/// PDA for an agent's bio, seeds = [b"bio", agent_record.key()].
/// Created / updated via `set_bio`. Dynamically sized — account is
/// reallocated on each update to fit the new bio content.
#[account]
pub struct AgentBio {
    /// Agent bio text, no max length (limited only by transaction size).
    pub bio: String,
}

impl AgentBio {
    /// Calculate space needed for a given bio length.
    pub fn space(bio_len: usize) -> usize {
        8 + 4 + bio_len // discriminator + String prefix + bytes
    }
}
