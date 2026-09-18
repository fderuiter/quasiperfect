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
    pub sigma_mod24: u32,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::UintExt;

    #[test]
    fn test_prefix_capture_and_restore_state() {
        let mut prefix = Prefix {
            n_l: Uint::from_u64(100),
            s_l: Uint::from_u64(200),
            last_idx: 5,
            factors: vec![3, 5, 7],
            sigma_factors: vec![Uint::from_u64(13), Uint::from_u64(31)],
            sigma_factors_u64: vec![13, 31],
            active_mask: vec![0b101, 0b010],
            sigma_mod24: 1,
        };

        let snap = prefix.capture_state();

        // Check captured snapshot
        assert_eq!(snap.n_l, Uint::from_u64(100));
        assert_eq!(snap.s_l, Uint::from_u64(200));
        assert_eq!(snap.last_idx, 5);
        assert_eq!(snap.factors_len, 3);
        assert_eq!(snap.sigma_factors_len, 2);
        assert_eq!(snap.sigma_factors_u64_len, 2);
        assert_eq!(snap.active_mask, vec![0b101, 0b010]);
        assert_eq!(snap.sigma_mod24, 1);

        // Mutate prefix state
        prefix.n_l = Uint::from_u64(999);
        prefix.s_l = Uint::from_u64(888);
        prefix.last_idx = 42;
        prefix.factors.push(11);
        prefix.factors.push(13);
        prefix.sigma_factors.push(Uint::from_u64(57));
        prefix.sigma_factors_u64.push(57);
        prefix.active_mask = vec![0b111, 0b111, 0b111];
        prefix.sigma_mod24 = 17;

        // Restore state
        prefix.restore_state(&snap);

        // Assert restored values
        assert_eq!(prefix.n_l, Uint::from_u64(100));
        assert_eq!(prefix.s_l, Uint::from_u64(200));
        assert_eq!(prefix.last_idx, 5);
        assert_eq!(prefix.factors, vec![3, 5, 7]);
        assert_eq!(
            prefix.sigma_factors,
            vec![Uint::from_u64(13), Uint::from_u64(31)]
        );
        assert_eq!(prefix.sigma_factors_u64, vec![13, 31]);
        assert_eq!(prefix.active_mask, vec![0b101, 0b010]);
        assert_eq!(prefix.sigma_mod24, 1);
    }
}
