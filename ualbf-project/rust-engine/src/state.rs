use crate::schema_generated::Prefix;
use crate::types::Uint;
use smallvec::SmallVec;

#[derive(Clone, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct BitSet {
    pub blocks: Vec<u64>,
}

impl BitSet {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub fn from_blocks(blocks: Vec<u64>) -> Self {
        Self { blocks }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            blocks: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const u64 {
        self.blocks.as_ptr()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&mut self) -> *mut u64 {
        self.blocks.as_mut_ptr()
    }

    /// Performs in-place bitwise AND with `mask`, recording cleared bits (diff) in `undo_diff`.
    #[inline(always)]
    pub fn intersect_with_undo(&mut self, mask: &[u64], undo_diff: &mut SmallVec<[u64; 16]>) {
        undo_diff.clear();
        let len = self.blocks.len().min(mask.len());
        for k in 0..len {
            let cleared = self.blocks[k] & !mask[k];
            undo_diff.push(cleared);
            self.blocks[k] &= mask[k];
        }
    }

    /// Reverts a previous `intersect_with_undo` operation by ORing back cleared bits.
    #[inline(always)]
    pub fn undo_intersect(&mut self, undo_diff: &[u64]) {
        let len = self.blocks.len().min(undo_diff.len());
        for k in 0..len {
            self.blocks[k] |= undo_diff[k];
        }
    }
}

impl std::ops::Deref for BitSet {
    type Target = [u64];
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.blocks
    }
}

impl std::ops::DerefMut for BitSet {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.blocks
    }
}

impl From<Vec<u64>> for BitSet {
    fn from(blocks: Vec<u64>) -> Self {
        Self { blocks }
    }
}

impl From<BitSet> for Vec<u64> {
    fn from(bs: BitSet) -> Self {
        bs.blocks
    }
}

pub struct PrefixStateSnapshot {
    pub n_l: Uint,
    pub s_l: Uint,
    pub last_idx: usize,
    pub factors_len: usize,
    pub sigma_factors_len: usize,
    pub sigma_factors_u64_len: usize,
    pub active_mask_diff: SmallVec<[u64; 16]>,
    pub sigma_mod24: u32,
}

impl Prefix {
    /// Captures the current state into a lightweight snapshot and intersects `active_mask` in-place
    /// with `mask_to_apply`, recording the cleared bits for zero-allocation undo.
    pub fn capture_state_and_intersect(&mut self, mask_to_apply: &[u64]) -> PrefixStateSnapshot {
        let mut active_mask_diff = SmallVec::new();
        self.active_mask
            .intersect_with_undo(mask_to_apply, &mut active_mask_diff);
        PrefixStateSnapshot {
            n_l: self.n_l,
            s_l: self.s_l,
            last_idx: self.last_idx,
            factors_len: self.factors.len(),
            sigma_factors_len: self.sigma_factors.len(),
            sigma_factors_u64_len: self.sigma_factors_u64.len(),
            active_mask_diff,
            sigma_mod24: self.sigma_mod24,
        }
    }

    /// Captures the current state of the prefix into a lightweight snapshot without mutating bitset.
    pub fn capture_state(&self) -> PrefixStateSnapshot {
        PrefixStateSnapshot {
            n_l: self.n_l,
            s_l: self.s_l,
            last_idx: self.last_idx,
            factors_len: self.factors.len(),
            sigma_factors_len: self.sigma_factors.len(),
            sigma_factors_u64_len: self.sigma_factors_u64.len(),
            active_mask_diff: SmallVec::new(),
            sigma_mod24: self.sigma_mod24,
        }
    }

    /// Restores the prefix state from a snapshot, truncating vectors and reverting bitset mutations in-place.
    pub fn restore_state(&mut self, snap: &PrefixStateSnapshot) {
        self.n_l = snap.n_l;
        self.s_l = snap.s_l;
        self.last_idx = snap.last_idx;
        self.factors.truncate(snap.factors_len);
        self.sigma_factors.truncate(snap.sigma_factors_len);
        self.sigma_factors_u64.truncate(snap.sigma_factors_u64_len);
        self.active_mask.undo_intersect(&snap.active_mask_diff);
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
            active_mask: BitSet::from_blocks(vec![0b101, 0b010]),
            sigma_mod24: 1,
        };

        let snap = prefix.capture_state_and_intersect(&[0b001, 0b010]);

        // Check captured snapshot metadata
        assert_eq!(snap.n_l, Uint::from_u64(100));
        assert_eq!(snap.s_l, Uint::from_u64(200));
        assert_eq!(snap.last_idx, 5);
        assert_eq!(snap.factors_len, 3);
        assert_eq!(snap.sigma_factors_len, 2);
        assert_eq!(snap.sigma_factors_u64_len, 2);
        assert_eq!(snap.active_mask_diff.as_slice(), &[0b100, 0b000]);
        assert_eq!(prefix.active_mask.blocks, vec![0b001, 0b010]);
        assert_eq!(snap.sigma_mod24, 1);

        // Mutate prefix state
        prefix.n_l = Uint::from_u64(999);
        prefix.s_l = Uint::from_u64(888);
        prefix.last_idx = 42;
        prefix.factors.push(11);
        prefix.factors.push(13);
        prefix.sigma_factors.push(Uint::from_u64(57));
        prefix.sigma_factors_u64.push(57);
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
        assert_eq!(prefix.active_mask.blocks, vec![0b101, 0b010]);
        assert_eq!(prefix.sigma_mod24, 1);
    }

    #[test]
    fn test_prefix_to_transport_non_null_ptrs() {
        let prefix_empty = Prefix {
            n_l: Uint::from_u64(100),
            s_l: Uint::from_u64(200),
            last_idx: 0,
            factors: vec![],
            sigma_factors: vec![],
            sigma_factors_u64: vec![],
            active_mask: BitSet::new(),
            sigma_mod24: 0,
        };
        let transport_empty = prefix_empty.to_transport();
        assert!(!transport_empty.sigma_factors.is_null());
        assert_eq!(transport_empty.sigma_factors_len, 0);

        let prefix_nonempty = Prefix {
            n_l: Uint::from_u64(100),
            s_l: Uint::from_u64(200),
            last_idx: 1,
            factors: vec![3],
            sigma_factors: vec![Uint::from_u64(13)],
            sigma_factors_u64: vec![13],
            active_mask: BitSet::from_blocks(vec![1]),
            sigma_mod24: 1,
        };
        let transport_nonempty = prefix_nonempty.to_transport();
        assert!(!transport_nonempty.sigma_factors.is_null());
        assert_eq!(transport_nonempty.sigma_factors_len, 1);
    }

    #[test]
    fn test_bitset_methods() {
        let bs_empty = BitSet::new();
        assert!(bs_empty.is_empty());
        assert_eq!(bs_empty.len(), 0);

        let mut bs_cap = BitSet::with_capacity(16);
        assert!(bs_cap.is_empty());
        assert_eq!(bs_cap.len(), 0);

        let mut bs = BitSet::from_blocks(vec![0b1111, 0b1010]);
        assert!(!bs.is_empty());
        assert_eq!(bs.len(), 2);

        assert!(!bs.as_ptr().is_null());
        assert!(!bs.as_mut_ptr().is_null());

        // Deref & DerefMut
        assert_eq!(bs[0], 0b1111);
        bs[0] = 0b1100;
        assert_eq!(bs[0], 0b1100);

        // Conversions
        let vec_blocks: Vec<u64> = vec![1, 2, 3];
        let bs_converted: BitSet = vec_blocks.into();
        assert_eq!(bs_converted.len(), 3);
        let back_vec: Vec<u64> = bs_converted.into();
        assert_eq!(back_vec, vec![1, 2, 3]);

        // Prefix capture_state
        let prefix = Prefix {
            n_l: Uint::from_u64(10),
            s_l: Uint::from_u64(20),
            last_idx: 1,
            factors: vec![3],
            sigma_factors: vec![Uint::from_u64(4)],
            sigma_factors_u64: vec![4],
            active_mask: BitSet::from_blocks(vec![0b111]),
            sigma_mod24: 0,
        };
        let snap = prefix.capture_state();
        assert_eq!(snap.n_l, Uint::from_u64(10));
        assert!(snap.active_mask_diff.is_empty());
    }
}
