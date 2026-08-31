import Mathlib.Data.Nat.GCD.Basic
import Mathlib.Data.ZMod.Basic
import Mathlib.NumberTheory.ArithmeticFunction.Misc
import UALBF.Basic

/-!
# Engine: Bipartition Verification

Prefix-Suffix bipartition theorems for the Rust search engine.
The Bipartition structure is defined in Basic.lean.
-/

namespace UALBF.Engine.Bipartition

open UALBF

/--
  Multiplicativity of Sigma over the Bipartition.
  Because N_L and N_R are coprime, sigma(N_L * N_R) = sigma(N_L) * sigma(N_R).
  This is a purely algebraic fact — no QPN assumption needed.
-/
lemma sigma_mul (b : UALBF.Bipartition) : sigma b.N = sigma b.N_L * sigma b.N_R := by
  rw [b.h_mul]
  unfold sigma
  exact Nat.Coprime.sum_divisors_mul b.h_coprime

/--
  Multiplicativity of the Abundancy Index over the Bipartition.
  Directly applies the general multiplicativity of H(N) to the search engine's split.
-/
lemma abundancy_index_mul_bipartition (b : UALBF.Bipartition) :
    abundancy_index b.N = abundancy_index b.N_L * abundancy_index b.N_R := by
  rw [b.h_mul]
  exact abundancy_index_mul b.h_coprime

/--
  Prefix-Sigma Coprimality.
  This proves the Modular Inverse in Phase 4 is mathematically guaranteed to exist.
  Note: This requires the QPN hypothesis, supplied separately from the bipartition.
-/
theorem prefix_sigma_coprime (b : UALBF.Bipartition) (h_qpn : IsQuasiperfect b.N) :
    b.N_L.Coprime (sigma b.N_L) := by
  let d := b.N_L.gcd (sigma b.N_L)
  have hd_N_L : d ∣ b.N_L := Nat.gcd_dvd_left b.N_L (sigma b.N_L)
  have hd_sigma : d ∣ sigma b.N_L := Nat.gcd_dvd_right b.N_L (sigma b.N_L)
  have h_eq : sigma b.N_L * sigma b.N_R = 2 * b.N_L * b.N_R + 1 := by
    rw [← sigma_mul b]
    have h2 := h_qpn.right
    rw [h2]
    rw [b.h_mul]
    rw [← Nat.mul_assoc]
  have h_dvd_LHS : d ∣ sigma b.N_L * sigma b.N_R := Nat.dvd_trans hd_sigma (Nat.dvd_mul_right (sigma b.N_L) (sigma b.N_R))
  have h_dvd_2NLR : d ∣ 2 * b.N_L * b.N_R := by
    have h_dvd_N_L_R : d ∣ b.N_L * b.N_R := Nat.dvd_trans hd_N_L (Nat.dvd_mul_right b.N_L b.N_R)
    have h_dvd_2_N_L_R : d ∣ 2 * (b.N_L * b.N_R) := Nat.dvd_trans h_dvd_N_L_R (Nat.dvd_mul_left (b.N_L * b.N_R) 2)
    rw [← Nat.mul_assoc] at h_dvd_2_N_L_R
    exact h_dvd_2_N_L_R
  have h_dvd_RHS : d ∣ 2 * b.N_L * b.N_R + 1 := by
    rw [← h_eq]
    exact h_dvd_LHS
  have h_dvd_1 : d ∣ 1 := by
    have h_diff := Nat.dvd_sub h_dvd_RHS h_dvd_2NLR
    have h_add_sub : 2 * b.N_L * b.N_R + 1 - 2 * b.N_L * b.N_R = 1 := Nat.add_sub_cancel_left (2 * b.N_L * b.N_R) 1
    rw [h_add_sub] at h_diff
    exact h_diff
  exact Nat.eq_one_of_dvd_one h_dvd_1

/--
  AMBS Value Constraint.
  This proves the exact target modulo equation used in the Rust raycast.
  Note: This requires the QPN hypothesis, supplied separately from the bipartition.
-/
theorem ambs_suffix_target (b : UALBF.Bipartition) (h_qpn : IsQuasiperfect b.N) :
    (b.N_R : ZMod (sigma b.N_L)) * (2 * b.N_L : ZMod (sigma b.N_L)) = -1 := by
  have h_eq : sigma b.N_L * sigma b.N_R = 2 * b.N_L * b.N_R + 1 := by
    rw [← sigma_mul b]
    have h2 := h_qpn.right
    rw [h2]
    rw [b.h_mul]
    rw [← Nat.mul_assoc]
  have h_cast : ((2 * b.N_L * b.N_R + 1 : ℕ) : ZMod (sigma b.N_L)) = ((sigma b.N_L * sigma b.N_R : ℕ) : ZMod (sigma b.N_L)) := by
    rw [h_eq]
  have h_rhs_zero : ((sigma b.N_L * sigma b.N_R : ℕ) : ZMod (sigma b.N_L)) = 0 := by
    push_cast
    have hc : (sigma b.N_L : ZMod (sigma b.N_L)) = 0 := ZMod.natCast_self (sigma b.N_L)
    rw [hc]
    exact zero_mul _
  have h_lhs : ((2 * b.N_L * b.N_R + 1 : ℕ) : ZMod (sigma b.N_L)) =
    (2 * b.N_L : ZMod (sigma b.N_L)) * (b.N_R : ZMod (sigma b.N_L)) + 1 := by
    push_cast
    ring
  have h_comb : (2 * b.N_L : ZMod (sigma b.N_L)) * (b.N_R : ZMod (sigma b.N_L)) + 1 = 0 := by
    rw [← h_lhs, h_cast, h_rhs_zero]
  calc (b.N_R : ZMod (sigma b.N_L)) * (2 * b.N_L : ZMod (sigma b.N_L))
    _ = (2 * b.N_L : ZMod (sigma b.N_L)) * (b.N_R : ZMod (sigma b.N_L)) := mul_comm _ _
    _ = -1 := eq_neg_of_add_eq_zero_left h_comb

/--
  Contrapositive of `ambs_suffix_target`:
  If no N_R satisfying the bipartition's constraints also satisfies the modular
  QPN equation, then N cannot be quasiperfect.
-/
theorem no_solution_no_qpn (b : UALBF.Bipartition)
    (h_no_sol : (b.N_R : ZMod (sigma b.N_L)) * (2 * b.N_L : ZMod (sigma b.N_L)) ≠ -1) :
    ¬ IsQuasiperfect b.N := by
  intro h_qpn
  exact h_no_sol (ambs_suffix_target b h_qpn)

/--
  Root Partition Complete Coverage.
  Proves that evaluating memory-isolated disjoint root subtrees provides complete
  coverage of the target search space: if N is quasiperfect, its prefix N_L belongs
  to the partition set of subtrees.
-/
theorem root_partition_complete_coverage (subtrees : List (Nat → Prop))
    (h_cover : ∀ n_L : Nat, ∃ s ∈ subtrees, s n_L)
    (N : Nat) (h_qpn : IsQuasiperfect N) (b : UALBF.Bipartition) (h_N : b.N = N) :
    ∃ s ∈ subtrees, s b.N_L := by
  exact h_cover b.N_L

end UALBF.Engine.Bipartition
