use crate::schema_generated::Prefix;
use crate::types::Uint;

pub struct PrefixStateSnapshot {
    pub n_l: Uint,
    pub s_l: Uint,
    pub last_idx: usize,
    pub factors_len: usize,
    pub sigma_factors_len: usize,
    pub sigma_factors_u64_len: usize,
    pub active_mask: Vec<u64>,
    pub sigma_mod24: u64,
}

impl Prefix {
    /// Captures the current state of the prefix into a lightweight snapshot,
    /// avoiding full allocations where possible.
    pub fn capture_state(&self) -> PrefixStateSnapshot {
        PrefixStateSnapshot {
            n_l: self.n_l,
            s_l: self.s_l,
            last_idx: self.last_idx,
            factors_len: self.factors.len(),
            sigma_factors_len: self.sigma_factors.len(),
            sigma_factors_u64_len: self.sigma_factors_u64.len(),
            active_mask: self.active_mask.clone(),
            sigma_mod24: self.sigma_mod24,
        }
    }

    /// Restores the prefix state from a snapshot, truncating vectors correctly.
    pub fn restore_state(&mut self, snap: &PrefixStateSnapshot) {
        self.n_l = snap.n_l;
        self.s_l = snap.s_l;
        self.last_idx = snap.last_idx;
        self.factors.truncate(snap.factors_len);
        self.sigma_factors.truncate(snap.sigma_factors_len);
        self.sigma_factors_u64.truncate(snap.sigma_factors_u64_len);
        self.active_mask = snap.active_mask.clone();
        self.sigma_mod24 = snap.sigma_mod24;
    }
}
