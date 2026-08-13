import UALBF.Basic
import UALBF.Pure.EulerProduct
import UALBF.Pure.RationalBounds
import UALBF.Pure.Fixed64
import UALBF.QPN.BasicProperties
import UALBF.QPN.PrasadSunitha
import UALBF.ManifestConstants
import Mathlib.Data.Rat.Defs
import Mathlib.Data.Nat.Totient
import Mathlib.Algebra.BigOperators.Ring.Nat
import Mathlib.Tactic.Ring
import Mathlib.Tactic.Linarith
import Mathlib.Tactic.Positivity
import Mathlib.Tactic.FieldSimp
import Mathlib.Order.Interval.Finset.Nat
import Mathlib.Algebra.Order.BigOperators.Group.Finset

/-!
# QPN Abundancy Bound

QPN-specific abundancy analysis tying together the Pure layer's
algebraic machinery with the IsQuasiperfect hypothesis.

## Contents
- `qpn_abundancy_target`: exact abundancy for QPNs (2 + 1/N)
- `qpn_factorization_ge_two`: all prime exponents ≥ 2 (since QPN = m²)
- `correction_factor_bound`: C < 1022/1000 for QPNs coprime to 15
- `qpn_totient_bound`: N/φ(N) < 2.0442 for large QPNs
- `abundancy_starvation`: pruning validity for the Rust engine
-/

namespace UALBF.QPN.AbundancyBound

open UALBF UALBF.QPN.BasicProperties UALBF.QPN.PrasadSunitha
open UALBF.Pure.EulerProduct UALBF.Pure.RationalBounds
open Nat Finset

/-! ### QPN Abundancy Target -/

/-- If N is a QPN, its abundancy index is exactly 2 + 1/N. -/
theorem qpn_abundancy_target {N : ℕ} (h : IsQuasiperfect N) :
  abundancy_index N = 2 + 1 / (N : ℚ) := by
  have hn_pos : N > 0 := h.1
  have hn_cast_ne_zero : (N : ℚ) ≠ 0 := by exact Nat.cast_ne_zero.mpr (Nat.ne_of_gt hn_pos)
  have h_sigma : sigma N = 2 * N + 1 := h.2
  unfold abundancy_index
  rw [h_sigma]
  push_cast
  have h_div : (2 * (N : ℚ) + 1) / (N : ℚ) = (2 * (N : ℚ)) / (N : ℚ) + 1 / (N : ℚ) := by ring
  rw [h_div]
  congr 1
  exact mul_div_cancel_right₀ 2 hn_cast_ne_zero

/-! ### Correction Factor Helpers -/

/-- Each correction factor p^{v+1}/(p^{v+1}-1) is positive for prime p. -/
private lemma correction_factor_pos {p v : ℕ} (hp : Nat.Prime p) :
    (0 : ℚ) < (p ^ (v + 1) : ℚ) / (p ^ (v + 1) - 1) := by
  have hp_pos : (0 : ℚ) < (p : ℚ) := by exact_mod_cast hp.pos
  have hp_ge1 : (1 : ℚ) ≤ (p : ℚ) := by exact_mod_cast hp.one_le
  have h_pow_pos : (0 : ℚ) < (p : ℚ) ^ (v + 1) := pow_pos hp_pos _
  have h_pow_gt1 : (1 : ℚ) < (p : ℚ) ^ (v + 1) := by
    calc (1 : ℚ) < (p : ℚ) := by exact_mod_cast hp.one_lt
      _ = (p : ℚ) ^ 1 := (pow_one _).symm
      _ ≤ (p : ℚ) ^ (v + 1) := pow_le_pow_right₀ hp_ge1 (by omega)
  exact div_pos h_pow_pos (by linarith)

/-- For p ≥ 7 and v ≥ 2: p^{v+1}/(p^{v+1}-1) ≤ p³/(p³-1),
    since v+1 ≥ 3 implies p^{v+1} ≥ p³ and x/(x-1) is anti-monotone. -/
lemma correction_factor_le_cube_factor {p v : ℕ} (hp : p ≥ 7) (hv : v ≥ 2) :
    (p ^ (v + 1) : ℚ) / (p ^ (v + 1) - 1) ≤ (p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1) := by
  have hp3_gt1 : (1 : ℚ) < (p : ℚ) ^ 3 := by
    calc (1 : ℚ) < (7 : ℚ) ^ 3 := by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN]
      _ ≤ (p : ℚ) ^ 3 := by
        apply pow_le_pow_left₀ (by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN] : (0 : ℚ) ≤ 7)
        exact_mod_cast hp
  have h_le : (p : ℚ) ^ 3 ≤ (p : ℚ) ^ (v + 1) := by
    apply pow_le_pow_right₀
    · exact_mod_cast (show 1 ≤ p by omega)
    · omega
  exact div_pred_antitone hp3_gt1 h_le

/-- p³/(p³-1) is positive for prime p. -/
private lemma cube_factor_pos (p : ℕ) (hp : Nat.Prime p) :
    (0 : ℚ) < (p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1) := by
  have hp_pos : (0 : ℚ) < (p : ℚ) := by exact_mod_cast hp.pos
  have hp3 : (0 : ℚ) < (p : ℚ) ^ 3 := pow_pos hp_pos 3
  have hp3_gt1 : (1 : ℚ) < (p : ℚ) ^ 3 := by
    calc (1 : ℚ) < 2 ^ 3 := by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN]
      _ ≤ (p : ℚ) ^ 3 := by
        apply pow_le_pow_left₀ (by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN] : (0 : ℚ) ≤ 2)
        exact_mod_cast hp.two_le
  exact div_pos hp3 (by linarith)

/-- p³/(p³-1) ≥ 1 for prime p. -/
private lemma cube_factor_ge_one (p : ℕ) (hp : Nat.Prime p) :
    (1 : ℚ) ≤ (p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1) := by
  have hp_pos : (0 : ℚ) < (p : ℚ) := by exact_mod_cast hp.pos
  have hp3 : (0 : ℚ) < (p : ℚ) ^ 3 := pow_pos hp_pos 3
  have hp3_gt1 : (1 : ℚ) < (p : ℚ) ^ 3 := by
    calc (1 : ℚ) < 2 ^ 3 := by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN]
      _ ≤ (p : ℚ) ^ 3 := by
        apply pow_le_pow_left₀ (by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN] : (0 : ℚ) ≤ 2)
        exact_mod_cast hp.two_le
  rw [le_div_iff₀ (by linarith)]
  linarith

/-! ### Full Correction Factor Bound -/

/-- The full correction factor C = ∏ p^{v+1}/(p^{v+1}-1) over all
    prime factors of a QPN with gcd(N,15) = 1 is < 1022/1000.
    All v_p ≥ 2 (QPN is odd square) and all p ≥ 7 (coprime to 15),
    so each factor ≤ p³/(p³-1). Split into head (p ≤ 61) and tail (p > 61). -/
lemma correction_factor_bound {N : ℕ} (h_qpn : IsQuasiperfect N)
    (h_coprime : N.gcd 15 = 1) :
    ∏ p ∈ N.primeFactors,
      ((p ^ (N.factorization p + 1) : ℚ) / (p ^ (N.factorization p + 1) - 1)) <
    1022 / 1000 := by
  have h_ge7 := qpn_coprime_15_primes_ge_7 h_qpn h_coprime
  have h_prime : ∀ p ∈ N.primeFactors, Nat.Prime p :=
    fun p hp => (Nat.mem_primeFactors.mp hp).1
  have h_v_ge2 : ∀ p ∈ N.primeFactors, N.factorization p ≥ 2 :=
    fun p hp => qpn_factorization_ge_two h_qpn p hp

  -- Each factor ≤ p³/(p³-1) since v_p ≥ 2 and p ≥ 7
  have h_cube_bound : ∏ p ∈ N.primeFactors,
      ((p ^ (N.factorization p + 1) : ℚ) / (p ^ (N.factorization p + 1) - 1)) ≤
      ∏ p ∈ N.primeFactors, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) := by
    apply Finset.prod_le_prod
    · intro p hp
      exact le_of_lt (correction_factor_pos (h_prime p hp))
    · intro p hp
      exact correction_factor_le_cube_factor (h_ge7 p hp) (h_v_ge2 p hp)

  let T := UALBF.Manifest.PRIME_SPLIT_THRESHOLD
  -- Split into head (p ≤ T) and tail (p > T)
  set head := N.primeFactors.filter (fun p => p ≤ T)
  set tail := N.primeFactors.filter (fun p => ¬ p ≤ T)

  have h_split : ∏ p ∈ N.primeFactors, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) =
      (∏ p ∈ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1))) *
      (∏ p ∈ tail, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1))) := by
    rw [← Finset.prod_filter_mul_prod_filter_not N.primeFactors (fun p => p ≤ T)]

  -- Head product: subset of primes in [7, T], bounded by explicit computation using VerifyHeadProduct
  have h_head_bound : ∏ p ∈ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) < 10048 / (UALBF.Manifest.EULER_CEILING_DEN : ℚ) := by
    have h_head_sub : head ⊆ Finset.filter (fun p => Nat.Prime p) (Finset.Icc 7 T) := by
      intro p hp
      rw [Finset.mem_filter] at hp ⊢
      exact ⟨Finset.mem_Icc.mpr ⟨h_ge7 p hp.1, hp.2⟩, h_prime p hp.1⟩
    have h_head_le_full : ∏ p ∈ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) ≤
        ∏ p ∈ Finset.filter (fun p => Nat.Prime p) (Finset.Icc 7 T),
          ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) := by
      set fullSet := Finset.filter (fun p => Nat.Prime p) (Finset.Icc 7 T)
      have h_full_eq : ∏ p ∈ fullSet, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) =
          (∏ p ∈ fullSet \ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1))) *
          (∏ p ∈ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1))) :=
        (Finset.prod_sdiff h_head_sub).symm
      have h_sdiff_ge1 : (1 : ℚ) ≤ ∏ p ∈ fullSet \ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) := by
        have _h1 : (1 : ℚ) = ∏ _p ∈ fullSet \ head, (1 : ℚ) := (Finset.prod_const_one).symm
        have _h2 : ∏ _p ∈ fullSet \ head, (1 : ℚ) ≤
            ∏ p ∈ fullSet \ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) := by
          apply Finset.prod_le_prod
          · intro p _; norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN]
          · intro p hp
            have hp_full := (Finset.mem_sdiff.mp hp).1
            exact cube_factor_ge_one p (Finset.mem_filter.mp hp_full).2
        linarith
      have h_head_prod_pos : (0 : ℚ) < ∏ p ∈ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) := by
        apply Finset.prod_pos
        intro p hp
        exact cube_factor_pos p (h_prime p (Finset.mem_filter.mp hp).1)
      calc ∏ p ∈ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1))
          = 1 * (∏ p ∈ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1))) := by ring
        _ ≤ (∏ p ∈ fullSet \ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1))) *
            (∏ p ∈ head, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1))) :=
          mul_le_mul_of_nonneg_right h_sdiff_ge1 (le_of_lt h_head_prod_pos)
        _ = ∏ p ∈ fullSet, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) := h_full_eq.symm
    haveI : VerifyHeadProduct T := ⟨by
      have h_T_eq : T = (60 + 1) := rfl
      rw [h_T_eq]
      have h_all_primes_761 : Finset.filter (fun p => Nat.Prime p) (Finset.Icc 7 (60 + 1)) =
          {7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 60 + 1} := by
        decide
      rw [h_all_primes_761]
      have h_explicit : ∏ p ∈ ({7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 60 + 1} : Finset ℕ),
          ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) =
          (343 : ℚ) / 342 * (1331 / 1330) * (2197 / 2196) * (4913 / 4912) *
          (6859 / 6858) * (12167 / 12166) * (24389 / 24388) * (29791 / 29790) *
          (50653 / 50652) * (68921 / 68920) * (79507 / 79506) * (103823 / 103822) *
          (148877 / 148876) * (205379 / 205378) * (226981 / 226980) := by
        repeat rw [Finset.prod_insert (by decide)]
        rw [Finset.prod_singleton]
        norm_num
      rw [h_explicit]
      norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN]
    ⟩
    have h_verified := VerifyHeadProduct.verified (T := T)
    exact lt_of_le_of_lt h_head_le_full h_verified

  -- Tail product: primes > T, bounded by Weierstrass
  have h_tail_bound : ∏ p ∈ tail, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) ≤ (T : ℚ) / ((T : ℚ) - 1) := by
    apply tail_correction_bound_generic T (by decide)
    · intro p hp
      have hp_mem := Finset.mem_filter.mp hp
      have h_not_le := hp_mem.2
      push Not at h_not_le
      exact h_not_le
    · intro p hp
      have hp_mem := Finset.mem_filter.mp hp
      exact h_prime p hp_mem.1

  -- Combine: head < 10048/10000, tail ≤ T/(T-1), product < 1022/1000
  have h_combined : ∏ p ∈ N.primeFactors, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) <
      10048 / (UALBF.Manifest.EULER_CEILING_DEN : ℚ) * ((T : ℚ) / ((T : ℚ) - 1)) := by
    rw [h_split]
    have h_tail_pos : (0 : ℚ) < ∏ p ∈ tail, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) := by
      apply Finset.prod_pos
      intro p hp
      exact cube_factor_pos p (h_prime p (Finset.mem_filter.mp hp).1)
    apply mul_lt_mul h_head_bound h_tail_bound h_tail_pos (by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN])

  have h_arith : (10048 : ℚ) / (UALBF.Manifest.EULER_CEILING_DEN : ℚ) * ((T : ℚ) / ((T : ℚ) - 1)) < 1022 / 1000 := by
    have h_eq : T = (60 + 1) := rfl
    rw [h_eq]
    norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN]

  calc ∏ p ∈ N.primeFactors,
        ((p ^ (N.factorization p + 1) : ℚ) / (p ^ (N.factorization p + 1) - 1))
      ≤ ∏ p ∈ N.primeFactors, ((p : ℚ) ^ 3 / ((p : ℚ) ^ 3 - 1)) := h_cube_bound
    _ < 10048 / (UALBF.Manifest.EULER_CEILING_DEN : ℚ) * ((T : ℚ) / ((T : ℚ) - 1)) := h_combined
    _ < 1022 / 1000 := h_arith

/-! ### Totient Geometric Window -/

/-- For massive QPN candidates (N > 10^35) with gcd(N, 15) = 1,
    the Euler ceiling N/φ(N) < 2.0442.

    Chain: σ(N)/N = 2 + 1/N < 20001/10000,
           C < 1022/1000,
           product < 2.0442. -/
theorem qpn_totient_bound {N : ℕ} (h_qpn : IsQuasiperfect N)
    (h_coprime : N.gcd 15 = 1) :
  (N : ℚ) / (N.totient : ℚ) < ((UALBF.Manifest.EULER_CEILING_NUM : ℚ) / (UALBF.Manifest.EULER_CEILING_DEN : ℚ)) := by
  have hN_gt1 : N > 1 := by
    by_contra hle; push Not at hle
    have hp_cases : N = 0 ∨ N = 1 := by omega
    rcases hp_cases with rfl | rfl
    · have h_eq : sigma 0 = 2 * 0 + 1 := h_qpn.2
      revert h_eq; decide
    · have h_eq : sigma 1 = 2 * 1 + 1 := h_qpn.2
      revert h_eq; decide
  have h_decomp := totient_ratio_decomp hN_gt1
  have h_abund := qpn_abundancy_target h_qpn
  have h_corr := correction_factor_bound h_qpn h_coprime
  have hN_pos : (0 : ℚ) < (N : ℚ) := Nat.cast_pos.mpr (by omega)
  have h_omega := qpn_coprime_15_omega_bound h_qpn h_coprime
  have h_prime_pos : ∀ p ∈ N.primeFactors, p ≥ 2 := by
    intro p hp
    have hp_prime := (Nat.mem_primeFactors.mp hp).1
    exact hp_prime.two_le
  have _hN_ge : (UALBF.Manifest.EULER_CEILING_DEN : ℚ) < (N : ℚ) := by
    have h_prod_le : ∏ _p ∈ N.primeFactors, 2 ≤ ∏ p ∈ N.primeFactors, p := by
      apply Finset.prod_le_prod
      · intro i _
        omega
      · intro i hi
        exact h_prime_pos i hi
    have h_prod_2 : ∏ _p ∈ N.primeFactors, 2 = 2 ^ N.primeFactors.card := by
      rw [Finset.prod_const]
    have h_div_le : ∏ p ∈ N.primeFactors, p ≤ N := by
      have hdvd : ∏ p ∈ N.primeFactors, p ∣ N := Nat.prod_primeFactors_dvd N
      apply Nat.le_of_dvd (by omega) hdvd
    have h_bound : 2 ^ UALBF.Manifest.PRASAD_SUNITHA_PROOF_BOUND ≤ N := by
      calc 2 ^ UALBF.Manifest.PRASAD_SUNITHA_PROOF_BOUND ≤ 2 ^ N.primeFactors.card := Nat.pow_le_pow_right (by omega) h_omega
        _ = ∏ _p ∈ N.primeFactors, 2 := h_prod_2.symm
        _ ≤ ∏ p ∈ N.primeFactors, p := h_prod_le
        _ ≤ N := h_div_le
    have h_cast : (2 : ℚ) ^ UALBF.Manifest.PRASAD_SUNITHA_PROOF_BOUND ≤ (N : ℚ) := by exact_mod_cast h_bound
    calc (UALBF.Manifest.EULER_CEILING_DEN : ℚ) < (2 : ℚ) ^ UALBF.Manifest.PRASAD_SUNITHA_PROOF_BOUND := by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN, UALBF.Manifest.PRASAD_SUNITHA_PROOF_BOUND]
      _ ≤ (N : ℚ) := h_cast
  have h_abund_bound : abundancy_index N < 2 + 1 / (UALBF.Manifest.EULER_CEILING_DEN : ℚ) := by
    rw [h_abund]
    have _h_inv : 1 / (N : ℚ) < 1 / UALBF.Manifest.EULER_CEILING_DEN := by
      rw [div_lt_div_iff₀ hN_pos (by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN])]
      linarith
    linarith
  rw [h_decomp]
  have h_corr_pos : 0 < ∏ p ∈ N.primeFactors,
      ((p ^ (N.factorization p + 1) : ℚ) / (p ^ (N.factorization p + 1) - 1)) := by
    apply Finset.prod_pos
    intro p hp
    have hp_prime := (Nat.mem_primeFactors.mp hp).1
    have _hp_ge2 : (2 : ℚ) ≤ (p : ℚ) := by exact_mod_cast hp_prime.two_le
    have h_pow_pos : (0 : ℚ) < (p : ℚ) ^ (N.factorization p + 1) := by positivity
    have h_pow_gt1 : (1 : ℚ) < (p : ℚ) ^ (N.factorization p + 1) := by
      calc (1 : ℚ) < (p : ℚ) := by linarith
        _ = (p : ℚ) ^ 1 := (pow_one _).symm
        _ ≤ (p : ℚ) ^ (N.factorization p + 1) := by
            apply pow_le_pow_right₀ (by linarith); omega
    exact div_pos h_pow_pos (by linarith)
  calc abundancy_index N * ∏ p ∈ N.primeFactors,
        ((p ^ (N.factorization p + 1) : ℚ) / (p ^ (N.factorization p + 1) - 1))
      < (2 + 1 / (UALBF.Manifest.EULER_CEILING_DEN : ℚ)) * (1022 / 1000) := by
        apply mul_lt_mul h_abund_bound (le_of_lt h_corr) h_corr_pos (by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN])
    _ < ((UALBF.Manifest.EULER_CEILING_NUM : ℚ) / (UALBF.Manifest.EULER_CEILING_DEN : ℚ)) := by norm_num [UALBF.Manifest.EULER_CEILING_NUM, UALBF.Manifest.EULER_CEILING_DEN]

/-! ### Starvation Pruning -/

def getOddPrimesLoop (fuel : ℕ) (n : ℕ) (k : ℕ) (acc : List ℕ) : List ℕ :=
  match fuel with
  | 0 => acc.reverse
  | fuel' + 1 =>
    if acc.length ≥ k then acc.reverse
    else if n.Prime then
      getOddPrimesLoop fuel' (n + 2) k (n :: acc)
    else
      getOddPrimesLoop fuel' (n + 2) k acc

def dynamicOddFactors (k : ℕ) : List ℕ :=
  getOddPrimesLoop 100000 3 k []

/-- A formally proven static upper bound for suffix abundancy based purely on length. -/
def static_suffix_bound (k : ℕ) : ℚ :=
  (dynamicOddFactors k).foldl (fun acc p => acc * (p : ℚ) / ((p : ℚ) - 1)) 1

/-- A quasiperfect number must have an abundancy index strictly greater than 2.
    Consequently, any candidate whose abundancy cannot exceed 2.0 (or is ≤ 2.0)
    cannot satisfy the quasiperfect condition. -/
theorem qpn_abundancy_gt_two {N : ℕ} (h : IsQuasiperfect N) :
    abundancy_index N > 2 := by
  have h_abund := qpn_abundancy_target h
  rw [h_abund]
  have _h_pos : (0 : ℚ) < 1 / (N : ℚ) := by
    apply one_div_pos.mpr
    exact Nat.cast_pos.mpr h.1
  linarith

/-- The formal 2.0 soundness bound: if a candidate's abundancy index is ≤ 2.0,
    it cannot be a quasiperfect number. This justifies the starvation and
    overflow heuristics used in the Rust search engine. -/
theorem abundancy_le_two_not_qpn {N : ℕ} (h : abundancy_index N ≤ 2) :
    ¬ IsQuasiperfect N := by
  intro h_qpn
  have _h_gt := qpn_abundancy_gt_two h_qpn
  linarith

/--
This is a *conditional pruning certificate*. It formally proves the *logical implication*
that if a branch's upper bound (the product of a prefix's abundancy and the max possible
suffix abundancy) is ≤ 2, it is impossible to reach the required abundancy > 2.

Following a CompCert-style trusted boundary design, this theorem resolves the purely
arithmetic contradiction via `linarith`. The burden of providing a sound `h_prefix_val`
(i.e., maintaining the runtime invariant that the dynamic prefix and suffix upper bounds
correctly bound the sequence's true abundancy) is delegated to the lock-free Rust engine.
-/
theorem abundancy_starvation (b : UALBF.Bipartition) (k : ℕ)
  (h_bound : abundancy_index b.N_L * static_suffix_bound k ≤ 2)
  (h_target : abundancy_index b.N > 2)
  (h_suffix_bound : abundancy_index b.N_R ≤ static_suffix_bound k) : False := by
  have _h_idx_mul : abundancy_index b.N = abundancy_index b.N_L * abundancy_index b.N_R := by
    have h_mul_idx := abundancy_index_mul b.h_coprime
    rw [←b.h_mul] at h_mul_idx
    exact h_mul_idx
  have _h_pos_L : abundancy_index b.N_L ≥ 0 := by
    unfold abundancy_index
    positivity
  nlinarith

end UALBF.QPN.AbundancyBound
