import UALBF.QPN.BasicProperties
import UALBF.Pure.RationalBounds
import UALBF.Pure.Zsigmondy
import UALBF.QPN.Obstruction
import UALBF.Pure.Cyclotomic
import UALBF.Pure.EulerProduct
import UALBF.Engine.SieveSoundness
import Mathlib.Data.Nat.Factorization.Basic
import UALBF.FFI
import UALBF.ManifestConstants

namespace UALBF.QPN.PrasadSunitha

open UALBF
open UALBF.QPN.BasicProperties
open UALBF.Pure.Zsigmondy
open UALBF.QPN.Obstruction
open UALBF.Pure.RationalBounds
open UALBF.Pure.Cyclotomic


theorem qpn_coprime_15_primes_ge_7 {N : ℕ} (h_qpn : IsQuasiperfect N) (h_coprime : N.Coprime 15) : ∀ p ∈ N.primeFactors, p ≥ 7 := by
  intro p hp
  have h_prime := Nat.prime_of_mem_primeFactors hp
  have hp_dvd := Nat.dvd_of_mem_primeFactors hp
  have _h_odd : Odd N := (qpn_is_odd_square h_qpn).1
  have _p_ne_2 : p ≠ 2 := by
    rintro rfl
    have h_even : 2 ∣ N := hp_dvd
    have hk := (qpn_is_odd_square h_qpn).1.choose_spec
    have h_not_even := Nat.two_not_dvd_two_mul_add_one (qpn_is_odd_square h_qpn).1.choose
    rw [← hk] at h_not_even
    exact h_not_even h_even
  have _p_ne_3 : p ≠ 3 := by
    rintro rfl
    have h3_dvd_15 : 3 ∣ 15 := ⟨5, rfl⟩
    have h3_dvd_gcd : 3 ∣ N.gcd 15 := Nat.dvd_gcd hp_dvd h3_dvd_15
    have h_gcd_eq_1 : N.gcd 15 = 1 := h_coprime
    rw [h_gcd_eq_1] at h3_dvd_gcd
    rcases h3_dvd_gcd with ⟨c, hc⟩
    omega
  have _p_ne_5 : p ≠ 5 := by
    rintro rfl
    have h5_dvd_15 : 5 ∣ 15 := ⟨3, rfl⟩
    have h5_dvd_gcd : 5 ∣ N.gcd 15 := Nat.dvd_gcd hp_dvd h5_dvd_15
    have h_gcd_eq_1 : N.gcd 15 = 1 := h_coprime
    rw [h_gcd_eq_1] at h5_dvd_gcd
    rcases h5_dvd_gcd with ⟨c, hc⟩
    omega
  have _p_ne_4 : p ≠ 4 := by rintro rfl; revert h_prime; decide
  have _p_ne_6 : p ≠ 6 := by rintro rfl; revert h_prime; decide
  have _p_ge_2 : p ≥ 2 := h_prime.two_le
  omega

noncomputable def P14 : Finset ℕ := {7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59}

lemma p14_le_59_all : ∀ p ∈ P14, p ≤ 59 := by decide
lemma p14_ge_2_all : ∀ p ∈ P14, p ≥ 2 := by decide
lemma p14_le_59 {p : ℕ} (h : p ∈ P14) : p ≤ 59 := p14_le_59_all p h
lemma p14_ge_2 {p : ℕ} (h : p ∈ P14) : p ≥ 2 := p14_ge_2_all p h

lemma p14_card : P14.card = 14 := by decide

lemma p14_prod_lt_2 : ∏ p ∈ P14, ((p : ℚ) / ((p : ℚ) - 1)) < 2 := by
  have h_eq : ∏ p ∈ P14, ((p : ℚ) / ((p : ℚ) - 1)) =
    (7:ℚ)/6 * (11/10) * (13/12) * (17/16) * (19/18) * (23/22) *
    (29/28) * (31/30) * (37/36) * (41/40) * (43/42) * (47/46) *
    (53/52) * (59/58) := by
    unfold P14
    repeat rw [Finset.prod_insert (by decide)]
    rw [Finset.prod_singleton]
    push_cast
    norm_num
  rw [h_eq]
  norm_num

lemma p_div_p_sub_one_le_generic (T : ℕ) (hT : T ≥ 2) {p : ℕ} (hp : p ≥ T) :
    (p : ℚ) / ((p : ℚ) - 1) ≤ (T : ℚ) / ((T : ℚ) - 1) := by
  have : (p : ℚ) ≥ (T : ℚ) := by exact_mod_cast hp
  have hp_pos : (0 : ℚ) < (p : ℚ) - 1 := by
    have : (1 : ℚ) < (p : ℚ) := by exact_mod_cast (show p > 1 by omega)
    linarith
  have h_mul_pos : (0 : ℚ) < (T : ℚ) - 1 := by
    have : (1 : ℚ) < (T : ℚ) := by exact_mod_cast (show T > 1 by omega)
    linarith
  rw [div_le_div_iff₀ hp_pos h_mul_pos]
  calc (p : ℚ) * ((T : ℚ) - 1)
    _ = (p : ℚ) * (T : ℚ) - (p : ℚ) := by ring
    _ ≤ (p : ℚ) * (T : ℚ) - (T : ℚ) := by linarith
    _ = (T : ℚ) * ((p : ℚ) - 1) := by ring

lemma p_div_p_sub_one_ge_generic (T : ℕ) (hT : T ≥ 2) {p : ℕ} (hp : p ≤ T - 2) (hp2 : p ≥ 2) :
    (T : ℚ) / ((T : ℚ) - 1) ≤ (p : ℚ) / ((p : ℚ) - 1) := by
  have : (p : ℚ) ≤ (T : ℚ) - 2 := by exact_mod_cast hp
  have hp_pos : (0 : ℚ) < (p : ℚ) - 1 := by
    have : (1 : ℚ) < (p : ℚ) := by exact_mod_cast (show p > 1 by omega)
    linarith
  have hT_pos : (0 : ℚ) < (T : ℚ) - 1 := by
    have : (1 : ℚ) < (T : ℚ) := by exact_mod_cast (show T > 1 by omega)
    linarith
  rw [div_le_div_iff₀ hT_pos hp_pos]
  calc (T : ℚ) * ((p : ℚ) - 1)
    _ = (T : ℚ) * (p : ℚ) - (T : ℚ) := by ring
    _ ≤ (T : ℚ) * (p : ℚ) - ((p : ℚ) + 2) := by linarith
    _ ≤ (p : ℚ) * (T : ℚ) - (p : ℚ) := by linarith
    _ = (p : ℚ) * ((T : ℚ) - 1) := by ring

lemma p_div_p_sub_one_ge_split_threshold {p : ℕ} (hp : p ≤ 59) (hp2 : p ≥ 2) :
    ((60 + 1 : ℕ) : ℚ) / 60 ≤ (p : ℚ) / ((p : ℚ) - 1) := by
  have h_generic := p_div_p_sub_one_ge_generic (60 + 1) (by omega) hp hp2
  have h_bound_eq : (((60 + 1 : ℕ) : ℕ) : ℚ) / ((((60 + 1 : ℕ) : ℕ) : ℚ) - 1) = ((60 + 1 : ℕ) : ℚ) / 60 := by norm_num
  rw [h_bound_eq] at h_generic
  exact h_generic

lemma p_div_p_sub_one_le_split_threshold {p : ℕ} (hp : p ≥ (60 + 1)) :
    (p : ℚ) / ((p : ℚ) - 1) ≤ ((60 + 1 : ℕ) : ℚ) / 60 := by
  have h_generic := p_div_p_sub_one_le_generic (60 + 1) (by omega) hp
  have h_bound_eq : (((60 + 1 : ℕ) : ℕ) : ℚ) / ((((60 + 1 : ℕ) : ℕ) : ℚ) - 1) = ((60 + 1 : ℕ) : ℚ) / 60 := by norm_num
  rw [h_bound_eq] at h_generic
  exact h_generic

lemma not_in_p14_ge_split_aux (T : ℕ) (hT : T = (60 + 1)) : ∀ p ∈ Finset.Icc 7 (T - 1), p.Prime → p ∈ P14 := by
  subst hT
  decide


theorem qpn_coprime_15_omega_bound {N : ℕ} (h_qpn : IsQuasiperfect N)
    (h_coprime : N.gcd 15 = 1) : UALBF.Manifest.PRASAD_SUNITHA_PROOF_BOUND ≤ N.primeFactors.card := by
  by_contra h_lt
  push Not at h_lt
  simp only [UALBF.Manifest.PRASAD_SUNITHA_PROOF_BOUND] at h_lt
  have h_ge7 : ∀ p ∈ N.primeFactors, p ≥ 7 := qpn_coprime_15_primes_ge_7 h_qpn h_coprime
  have h_prime : ∀ p ∈ N.primeFactors, p.Prime := fun p hp => (Nat.mem_primeFactors.mp hp).1

  let T := UALBF.Manifest.PRIME_SPLIT_THRESHOLD
  have h_T_eq : T = (60 + 1) := rfl

  set head := N.primeFactors.filter (fun p => p ≤ T - 1)
  set tail := N.primeFactors.filter (fun p => ¬ p ≤ T - 1)

  have h_split : ∏ p ∈ N.primeFactors, ((p : ℚ) / ((p : ℚ) - 1)) =
      (∏ p ∈ head, ((p : ℚ) / ((p : ℚ) - 1))) * (∏ p ∈ tail, ((p : ℚ) / ((p : ℚ) - 1))) := by
    rw [← Finset.prod_filter_mul_prod_filter_not N.primeFactors (fun p => p ≤ T - 1)]

  have h_head_sub : head ⊆ P14 := by
    intro p hp
    rw [Finset.mem_filter] at hp
    have hp_ge_7 := h_ge7 p hp.1
    have h_p := h_prime p hp.1
    exact not_in_p14_ge_split_aux T h_T_eq p (Finset.mem_Icc.mpr ⟨hp_ge_7, hp.2⟩) h_p

  have _h_tail_card : tail.card = N.primeFactors.card - head.card := by
    have : head.card + tail.card = N.primeFactors.card := by
      exact Finset.card_filter_add_card_filter_not (fun p => p ≤ T - 1)
    omega
  have h_tail_le : tail.card ≤ 14 - head.card := by omega

  have h_tail_bound : ∏ p ∈ tail, ((p : ℚ) / ((p : ℚ) - 1)) ≤ (T / (T - 1) : ℚ) ^ tail.card := by
    have h_eq : (T / (T - 1) : ℚ) ^ tail.card = ∏ _p ∈ tail, (T / (T - 1) : ℚ) := by rw [Finset.prod_const]
    rw [h_eq]
    apply Finset.prod_le_prod
    · intro p hp
      have hp_pos : (0 : ℚ) < (p : ℚ) - 1 := by
        have : p ≥ 7 := h_ge7 p (Finset.mem_filter.mp hp).1
        have hp_gt : (1 : ℚ) < (p : ℚ) := by exact_mod_cast (show 1 < p by omega)
        linarith
      exact le_of_lt (div_pos (by exact_mod_cast (show p > 0 by have := h_ge7 p (Finset.mem_filter.mp hp).1; omega)) hp_pos)
    · intro p hp
      have _hp_not : ¬ p ≤ T - 1 := (Finset.mem_filter.mp hp).2
      have hp_ge_T : p ≥ T := by omega
      have h_ge : (p : ℚ) / ((p : ℚ) - 1) ≤ (T : ℚ) / ((T : ℚ) - 1) := by
        apply p_div_p_sub_one_le_generic T (by decide) hp_ge_T
      exact h_ge

  have h_tail_bound2 : (T / (T - 1) : ℚ) ^ tail.card ≤ (T / (T - 1) : ℚ) ^ (14 - head.card) := by
    apply pow_le_pow_right₀ (by norm_num [h_T_eq]) h_tail_le

  have h_tail_bound3 : ∏ p ∈ tail, ((p : ℚ) / ((p : ℚ) - 1)) ≤ (T / (T - 1) : ℚ) ^ (14 - head.card) :=
    le_trans h_tail_bound h_tail_bound2

  have h_sdiff_card : (P14 \ head).card = 14 - head.card := by
    rw [Finset.card_sdiff_of_subset h_head_sub, p14_card]
  have h_sdiff_bound : (T / (T - 1) : ℚ) ^ (14 - head.card) ≤ ∏ p ∈ P14 \ head, ((p : ℚ) / ((p : ℚ) - 1)) := by
    rw [← h_sdiff_card]
    have h_eq : (T / (T - 1) : ℚ) ^ (P14 \ head).card = ∏ _p ∈ P14 \ head, (T / (T - 1) : ℚ) := by rw [Finset.prod_const]
    rw [h_eq]
    apply Finset.prod_le_prod
    · intro p hp; norm_num [h_T_eq]
    · intro p hp
      have hp_in : p ∈ P14 := (Finset.mem_sdiff.mp hp).1
      have hp_le59 : p ≤ 59 := p14_le_59 hp_in
      have hp_ge2 : p ≥ 2 := p14_ge_2 hp_in
      have hp_le_T_minus_2 : p ≤ T - 2 := by omega
      have h_ge : (T : ℚ) / ((T : ℚ) - 1) ≤ (p : ℚ) / ((p : ℚ) - 1) := by
        apply p_div_p_sub_one_ge_generic T (by decide) hp_le_T_minus_2 hp_ge2
      exact h_ge

  have h_tail_bound4 : ∏ p ∈ tail, ((p : ℚ) / ((p : ℚ) - 1)) ≤ ∏ p ∈ P14 \ head, ((p : ℚ) / ((p : ℚ) - 1)) :=
    le_trans h_tail_bound3 h_sdiff_bound

  have h_combined_bound : (∏ p ∈ head, ((p : ℚ) / ((p : ℚ) - 1))) * (∏ p ∈ tail, ((p : ℚ) / ((p : ℚ) - 1)))
      ≤ (∏ p ∈ head, ((p : ℚ) / ((p : ℚ) - 1))) * (∏ p ∈ P14 \ head, ((p : ℚ) / ((p : ℚ) - 1))) := by
    apply mul_le_mul_of_nonneg_left h_tail_bound4
    apply Finset.prod_nonneg
    intro p hp
    have hp_pos : (0 : ℚ) < (p : ℚ) - 1 := by
      have : p ≥ 7 := h_ge7 p (Finset.mem_filter.mp hp).1
      have hp_gt : (1 : ℚ) < (p : ℚ) := by exact_mod_cast (show 1 < p by omega)
      linarith
    exact le_of_lt (div_pos (by exact_mod_cast (show p > 0 by have := h_ge7 p (Finset.mem_filter.mp hp).1; omega)) hp_pos)

  have h_prod_eq : (∏ p ∈ head, ((p : ℚ) / ((p : ℚ) - 1))) * (∏ p ∈ P14 \ head, ((p : ℚ) / ((p : ℚ) - 1))) =
      ∏ p ∈ P14, ((p : ℚ) / ((p : ℚ) - 1)) := by
    calc
      _ = (∏ p ∈ P14 \ head, ((p : ℚ) / ((p : ℚ) - 1))) * (∏ p ∈ head, ((p : ℚ) / ((p : ℚ) - 1))) := by ring
      _ = ∏ p ∈ P14, ((p : ℚ) / ((p : ℚ) - 1)) := Finset.prod_sdiff h_head_sub

  have h_tot_bound : ∏ p ∈ N.primeFactors, ((p : ℚ) / ((p : ℚ) - 1)) ≤ ∏ p ∈ P14, ((p : ℚ) / ((p : ℚ) - 1)) := by
    rw [h_split]
    exact le_trans h_combined_bound (le_of_eq h_prod_eq)

  have h_tot_lt_2 : ∏ p ∈ N.primeFactors, ((p : ℚ) / ((p : ℚ) - 1)) < 2 :=
    lt_of_le_of_lt h_tot_bound p14_prod_lt_2

  have hN_gt1 : N > 1 := by
    by_contra hle; push Not at hle
    have hp_cases : N = 0 ∨ N = 1 := by omega
    rcases hp_cases with rfl | rfl
    · have h_eq : sigma 0 = 2 * 0 + 1 := h_qpn.2; revert h_eq; decide
    · have h_eq : sigma 1 = 2 * 1 + 1 := h_qpn.2; revert h_eq; decide

  have h_decomp := UALBF.Pure.EulerProduct.totient_ratio_decomp hN_gt1
  have h_abund : abundancy_index N = 2 + 1 / (N : ℚ) := by
    have hn_pos : N > 0 := h_qpn.1
    have hn_cast_ne_zero : (N : ℚ) ≠ 0 := Nat.cast_ne_zero.mpr (Nat.ne_of_gt hn_pos)
    have h_sigma : sigma N = 2 * N + 1 := h_qpn.2
    unfold abundancy_index
    rw [h_sigma]
    push_cast
    have h_div : (2 * (N : ℚ) + 1) / (N : ℚ) = (2 * (N : ℚ)) / (N : ℚ) + 1 / (N : ℚ) := by ring
    rw [h_div]
    congr 1
    exact mul_div_cancel_right₀ 2 hn_cast_ne_zero

  have h_corr_ge1 : 1 ≤ ∏ p ∈ N.primeFactors, ((p ^ (N.factorization p + 1) : ℚ) / (p ^ (N.factorization p + 1) - 1)) := by
    have h1 : (1 : ℚ) = ∏ _p ∈ N.primeFactors, (1 : ℚ) := Finset.prod_const_one.symm
    rw [h1]
    apply Finset.prod_le_prod
    · intro p hp; norm_num
    · intro p hp
      have hp_prime := h_prime p hp
      have hp_ge2 : p ≥ 2 := hp_prime.two_le
      have _hp_pow_gt1 : (1 : ℚ) < (p : ℚ) ^ (N.factorization p + 1) := by
        calc (1 : ℚ) < (p : ℚ) := by exact_mod_cast (show 1 < p by omega)
             _ = (p : ℚ) ^ 1 := (pow_one _).symm
             _ ≤ (p : ℚ) ^ (N.factorization p + 1) := by apply pow_le_pow_right₀ (by exact_mod_cast (show 1 ≤ p by omega)); omega
      rw [le_div_iff₀ (by linarith)]
      linarith

  have h_abund_le : abundancy_index N ≤ (N : ℚ) / (N.totient : ℚ) := by
    rw [h_decomp]
    have h_abund_pos : 0 ≤ abundancy_index N := by unfold abundancy_index; positivity
    calc abundancy_index N = abundancy_index N * 1 := by ring
         _ ≤ abundancy_index N * ∏ p ∈ N.primeFactors, ((p ^ (N.factorization p + 1) : ℚ) / (p ^ (N.factorization p + 1) - 1)) := by
           apply mul_le_mul_of_nonneg_left h_corr_ge1 h_abund_pos

  have h_N_phi : (N : ℚ) / (N.totient : ℚ) = ∏ p ∈ N.primeFactors, ((p : ℚ) / ((p : ℚ) - 1)) := by
    have hT_pos : (0 : ℚ) < (N.totient : ℚ) := Nat.cast_pos.mpr (Nat.totient_pos.mpr (show 0 < N by omega))
    have hT_ne_zero : (N.totient : ℚ) ≠ 0 := ne_of_gt hT_pos
    have h_id := Nat.totient_mul_prod_primeFactors N
    have h_id_q : (N.totient : ℚ) * (∏ p ∈ N.primeFactors, (p : ℚ)) =
        (N : ℚ) * (∏ p ∈ N.primeFactors, ((p : ℚ) - 1)) := by
      have h_cast_pred : (↑(∏ p ∈ N.primeFactors, (p - 1)) : ℚ) =
          (∏ p ∈ N.primeFactors, ((p : ℚ) - 1)) := by
        rw [Nat.cast_prod]
        apply Finset.prod_congr rfl
        intro p hp
        have hp_prime := h_prime p hp
        have hp_ge : 1 ≤ p := by have := hp_prime.two_le; omega
        rw [Nat.cast_sub hp_ge, Nat.cast_one]
      have h_cast_id : (↑(N.totient * ∏ p ∈ N.primeFactors, p) : ℚ) =
          (↑(N * ∏ p ∈ N.primeFactors, (p - 1)) : ℚ) := by
        exact_mod_cast h_id
      rw [Nat.cast_mul, Nat.cast_prod] at h_cast_id
      rw [Nat.cast_mul, h_cast_pred] at h_cast_id
      exact h_cast_id
    have h_prod_pred_ne : (∏ p ∈ N.primeFactors, ((p : ℚ) - 1)) ≠ 0 := by
      rw [Finset.prod_ne_zero_iff]
      intro p hp
      have hp_prime := h_prime p hp
      have : (2 : ℚ) ≤ (p : ℚ) := by exact_mod_cast hp_prime.two_le
      linarith
    rw [Finset.prod_div_distrib]
    rw [div_eq_div_iff hT_ne_zero h_prod_pred_ne]
    linarith [h_id_q]

  have h_abund_le_prod : abundancy_index N ≤ ∏ p ∈ N.primeFactors, ((p : ℚ) / ((p : ℚ) - 1)) := by
    rw [← h_N_phi]
    exact h_abund_le

  have _h_abund_lt_2 : abundancy_index N < 2 := lt_of_le_of_lt h_abund_le_prod h_tot_lt_2

  have _h_abund_gt_2 : abundancy_index N > 2 := by
    rw [h_abund]
    have hN_pos : (0 : ℚ) < (N : ℚ) := by exact_mod_cast (show N > 0 by omega)
    have _h_inv_pos : (0 : ℚ) < 1 / (N : ℚ) := one_div_pos.mpr hN_pos
    linarith

  linarith


/-- For a QPN (which is an odd square m²), every prime in its factorization
    has exponent ≥ 2 (all exponents are even, and membership ensures ≥ 1). -/
lemma qpn_factorization_ge_two {N : ℕ} (h_qpn : IsQuasiperfect N)
    (p : ℕ) (hp : p ∈ N.primeFactors) :
    N.factorization p ≥ 2 := by
  have ⟨_, m, hm⟩ := qpn_is_odd_square h_qpn
  have _hm_ne : m ≠ 0 := by intro h; rw [h] at hm; exact Nat.ne_of_gt h_qpn.1 hm
  have hm_sq : N.factorization p = 2 * m.factorization p := by
    rw [hm, Nat.factorization_pow]
    simp [Finsupp.coe_smul, Pi.smul_apply, smul_eq_mul]
  have _h_ge1 : N.factorization p ≥ 1 :=
    Nat.one_le_iff_ne_zero.mpr (Finsupp.mem_support_iff.mp hp)
  omega

/-- For a QPN (which is an odd square m²), every prime in its factorization
    has an even exponent. -/
lemma qpn_factorization_even {N : ℕ} (h_qpn : IsQuasiperfect N) (p : ℕ) :
    2 ∣ N.factorization p := by
  have ⟨_, m, hm⟩ := qpn_is_odd_square h_qpn
  have hm_sq : N.factorization p = 2 * m.factorization p := by
    rw [hm, Nat.factorization_pow]
    simp [Finsupp.coe_smul, Pi.smul_apply, smul_eq_mul]
  exact ⟨m.factorization p, hm_sq⟩

theorem val_11_ge_4 {N : ℕ} (h_qpn : IsQuasiperfect N) (h_11 : 11 ∈ N.primeFactors) : N.factorization 11 ≥ 4 := by
  have h_ge_2 := qpn_factorization_ge_two h_qpn 11 h_11
  have h_even := qpn_factorization_even h_qpn 11
  by_contra h_lt
  push Not at h_lt
  have h2 : N.factorization 11 = 2 := by
    rcases h_even with ⟨k, hk⟩
    omega
  have hp : Nat.Prime 11 := by decide
  have hN : N ≠ 0 := h_qpn.1.ne'
  have h_div : 11 ^ 2 ∣ N := (hp.pow_dvd_iff_le_factorization hN).mpr (by omega)
  have h_ndiv : ¬ (11 ^ 3 ∣ N) := by
    intro h
    have h_le := (hp.pow_dvd_iff_le_factorization hN).mp h
    omega
  have h_exact : ExactValuation 11 (2 * 1) N := ⟨h_div, h_ndiv⟩
  have hq : Nat.Prime 7 := by decide
  have hq_odd : 7 ≠ 2 := by decide
  have h_mod : 7 % 8 = 5 ∨ 7 % 8 = 7 := Or.inr (by decide)
  have h_sigma_eq : sigma (11 ^ 2) = sigma_prime_pow 11 1 := sigma_eq_sigma_prime_pow 11 1 hp
  have h_div_sig : 7 ∣ sigma (11 ^ (2 * 1)) := by
    change 7 ∣ sigma (11 ^ 2)
    rw [h_sigma_eq]
    decide
  exact UALBF.Engine.SieveSoundness.rust_sieve_soundness h_qpn hp hq hq_odd h_mod h_div_sig h_exact

theorem val_13_ge_4 {N : ℕ} (h_qpn : IsQuasiperfect N) (h_13 : 13 ∈ N.primeFactors) : N.factorization 13 ≥ 4 := by
  have h_ge_2 := qpn_factorization_ge_two h_qpn 13 h_13
  have h_even := qpn_factorization_even h_qpn 13
  by_contra h_lt
  push Not at h_lt
  have h2 : N.factorization 13 = 2 := by
    rcases h_even with ⟨k, hk⟩
    omega
  have hp : Nat.Prime 13 := by decide
  have hN : N ≠ 0 := h_qpn.1.ne'
  have h_div : 13 ^ (2 * 1) ∣ N := (hp.pow_dvd_iff_le_factorization hN).mpr (by omega)
  have h_ndiv : ¬ (13 ^ (2 * 1 + 1) ∣ N) := by
    intro h
    have h_le := (hp.pow_dvd_iff_le_factorization hN).mp h
    omega
  have h_exact : ExactValuation 13 (2 * 1) N := ⟨h_div, h_ndiv⟩
  let q := UALBF.Manifest.PRIME_SPLIT_THRESHOLD
  have h_T_eq : q = (60 + 1) := rfl
  have hq : Nat.Prime q := by
    rw [h_T_eq]
    decide
  have hq_odd : q ≠ 2 := by
    rw [h_T_eq]
    decide
  have h_mod : q % 8 = 5 ∨ q % 8 = 7 := by
    rw [h_T_eq]
    decide
  have h_sigma_eq : sigma (13 ^ 2) = sigma_prime_pow 13 1 := sigma_eq_sigma_prime_pow 13 1 hp
  have h_div_sig : q ∣ sigma (13 ^ (2 * 1)) := by
    change q ∣ sigma (13 ^ 2)
    rw [h_sigma_eq, h_T_eq]
    decide
  exact UALBF.Engine.SieveSoundness.rust_sieve_soundness h_qpn hp hq hq_odd h_mod h_div_sig h_exact

end UALBF.QPN.PrasadSunitha
