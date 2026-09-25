import Mathlib.NumberTheory.LegendreSymbol.QuadraticReciprocity
import Mathlib.Data.ZMod.Basic
import UALBF.QPN.BasicProperties

/-!
# QPN Obstruction: Legendre-Cattaneo Mod-8 Filter

The universal modulo-8 obstruction for sigma divisors of QPNs.
-/

namespace UALBF.QPN.Obstruction

open UALBF
open UALBF.QPN.BasicProperties

theorem qpn_mod_3_eq_1 {n : ℕ} (h_qpn : IsQuasiperfect n) (h_3_dvd : 3 ∣ n) :
  sigma n % 3 = 1 := by
  have ⟨_, m, hm_sq⟩ := qpn_is_odd_square h_qpn
  have h_3_dvd_m : 3 ∣ m := Nat.Prime.dvd_of_dvd_pow Nat.prime_three (by rw [← hm_sq]; exact h_3_dvd)
  have h_sigma : sigma n = 2 * m ^ 2 + 1 := by rw [h_qpn.2, hm_sq]
  have h_decide : ∀ (x : ZMod 3), x = 0 → 2 * x ^ 2 + 1 = 1 := by decide
  have h_m_zmod : (m : ZMod 3) = 0 := by
    rw [←ZMod.natCast_mod m 3, Nat.mod_eq_zero_of_dvd h_3_dvd_m]
    rfl
  have h_sigma_zmod : (sigma n : ZMod 3) = 1 := by
    push_cast [h_sigma]
    exact h_decide (m : ZMod 3) h_m_zmod
  have h_val : (sigma n : ZMod 3).val = (1 : ZMod 3).val := by rw [h_sigma_zmod]
  rw [ZMod.val_natCast] at h_val
  exact h_val

theorem qpn_mod_9_eq_1 {n : ℕ} (h_qpn : IsQuasiperfect n) (h_9_dvd : 9 ∣ n) :
  sigma n % 9 = 1 := by
  have ⟨_, m, hm_sq⟩ := qpn_is_odd_square h_qpn
  have h_3_dvd_m : 3 ∣ m := by
    have h_3_dvd_sq : 3 ∣ m ^ 2 := by
      have h9 : 3 ∣ 9 := by decide
      exact dvd_trans h9 (by rw [← hm_sq]; exact h_9_dvd)
    exact Nat.Prime.dvd_of_dvd_pow Nat.prime_three h_3_dvd_sq
  have h_sigma : sigma n = 2 * m ^ 2 + 1 := by rw [h_qpn.2, hm_sq]
  have h_decide : ∀ (x : ZMod 9), (x.val % 3 = 0) → 2 * x ^ 2 + 1 = 1 := by decide
  have h_m_val : (m : ZMod 9).val % 3 = 0 := by
    rw [ZMod.val_natCast]
    have _h_mod_3 : m % 3 = 0 := Nat.mod_eq_zero_of_dvd h_3_dvd_m
    omega
  have h_sigma_zmod : (sigma n : ZMod 9) = 1 := by
    push_cast [h_sigma]
    exact h_decide (m : ZMod 9) h_m_val
  have h_val : (sigma n : ZMod 9).val = (1 : ZMod 9).val := by rw [h_sigma_zmod]
  rw [ZMod.val_natCast] at h_val
  exact h_val

/--
  The Universal Modulo-8 Obstruction (Legendre-Cattaneo).
  If q is an odd prime factor of σ(N) for a quasiperfect N, then q ≡ 1 or 3 (mod 8).
-/
theorem legendre_cattaneo_obstruction {n q : ℕ}
  (h_qpn : IsQuasiperfect n)
  (hq_prime : q.Prime)
  (hq_odd : q ≠ 2)
  (h_div : q ∣ sigma n) :
  q % 8 = 1 ∨ q % 8 = 3 := by
  have ⟨_h_odd, m, hm_sq⟩ := qpn_is_odd_square h_qpn
  have h_sigma_sq : sigma (m ^ 2) = 2 * m ^ 2 + 1 := by
    have h_qpn_eq := h_qpn.2
    rw [hm_sq] at h_qpn_eq
    exact h_qpn_eq

  have h_q_div : q ∣ 2 * m ^ 2 + 1 := by
    have h_div_sq : q ∣ sigma (m ^ 2) := by
      rw [←hm_sq]
      exact h_div
    rw [h_sigma_sq] at h_div_sq
    exact h_div_sq

  have h_mod_0 : ((2 * m ^ 2 + 1 : ℕ) : ZMod q) = 0 := by
    rcases h_q_div with ⟨c, hc⟩
    rw [hc, Nat.cast_mul, CharP.cast_eq_zero (ZMod q) q, zero_mul]

  have h_eq_2 : (2 : ZMod q) * (m : ZMod q) ^ 2 + 1 = 0 := by
    calc (2 : ZMod q) * (m : ZMod q) ^ 2 + 1 = ((2 * m ^ 2 + 1 : ℕ) : ZMod q) := by push_cast; rfl
         _ = 0 := h_mod_0

  have h_eq_4 : (4 : ZMod q) * (m : ZMod q) ^ 2 + 2 = 0 := by
    calc (4 : ZMod q) * (m : ZMod q) ^ 2 + 2 = 2 * ((2 : ZMod q) * (m : ZMod q) ^ 2 + 1) := by ring
         _ = 2 * 0 := by rw [h_eq_2]
         _ = 0 := mul_zero 2

  have h_sq_neg2 : ((2 * m : ℕ) : ZMod q) ^ 2 = -2 := by
    calc ((2 * m : ℕ) : ZMod q) ^ 2 = (4 : ZMod q) * (m : ZMod q) ^ 2 := by push_cast; ring
         _ = (4 : ZMod q) * (m : ZMod q) ^ 2 + 2 - 2 := by ring
         _ = 0 - 2 := by rw [h_eq_4]
         _ = -2 := by ring

  have h_is_sq : IsSquare ((-2 : ℤ) : ZMod q) := by
    use ((2 * m : ℕ) : ZMod q)
    have h_int_cast : ((-2 : ℤ) : ZMod q) = -2 := by push_cast; rfl
    rw [h_int_cast]
    calc -2 = ((2 * m : ℕ) : ZMod q) ^ 2 := h_sq_neg2.symm
         _ = ((2 * m : ℕ) : ZMod q) * ((2 * m : ℕ) : ZMod q) := by ring

  haveI : Fact q.Prime := ⟨hq_prime⟩

  have h_int_cast : ((-2 : ℤ) : ZMod q) = -2 := by push_cast; rfl

  have hq_n2_ne_zero : ((-2 : ℤ) : ZMod q) ≠ 0 := by
    rw [h_int_cast]
    intro h_zero
    have h_2_zero : (2 : ZMod q) = 0 := by
      calc (2 : ZMod q) = -(-2 : ZMod q) := by ring
           _ = -0 := by rw [h_zero]
           _ = 0 := neg_zero
    have h_q_dvd_2 : q ∣ 2 := (CharP.cast_eq_zero_iff (ZMod q) q 2).mp h_2_zero
    have _h_q_le_2 : q ≤ 2 := Nat.le_of_dvd (by decide) h_q_dvd_2
    have h_q_eq_2 : q = 2 := by
      have _h_q_ge_2 : q ≥ 2 := Nat.Prime.two_le hq_prime
      omega
    exact hq_odd h_q_eq_2

  have h_leg_eq_1 : legendreSym q (-2 : ℤ) = 1 := by
    rw [legendreSym.eq_one_iff q hq_n2_ne_zero]
    exact h_is_sq

  have h_leg_eq_chi : legendreSym q (-2 : ℤ) = ZMod.χ₈' (q : ZMod 8) := legendreSym.at_neg_two hq_odd

  have h_chi_1 : ZMod.χ₈' (q : ZMod 8) = 1 := by
    rw [←h_leg_eq_chi]
    exact h_leg_eq_1

  have h_decide : ∀ (x : ZMod 8), ZMod.χ₈' x = 1 → x = 1 ∨ x = 3 := by decide
  have h_or := h_decide (q : ZMod 8) h_chi_1
  rcases h_or with h1 | h3
  · left
    have h_val : (q : ZMod 8).val = (1 : ZMod 8).val := by rw [h1]
    rw [ZMod.val_natCast] at h_val
    exact h_val
  · right
    have h_val : (q : ZMod 8).val = (3 : ZMod 8).val := by rw [h3]
    rw [ZMod.val_natCast] at h_val
    exact h_val


theorem qpn_mod_4_eq_3 {N : ℕ} (h_qpn : IsQuasiperfect N) : sigma N % 4 = 3 := by
  have ⟨h_odd, m, hm_sq⟩ := qpn_is_odd_square h_qpn
  have h_sigma : sigma N = 2 * m ^ 2 + 1 := by rw [h_qpn.2, hm_sq]
  have h_decide : ∀ (x : ZMod 4), x.val % 2 = 1 → 2 * x ^ 2 + 1 = 3 := by decide
  have h_m_val : (m : ZMod 4).val % 2 = 1 := by
    rw [ZMod.val_natCast]
    have hn1 : N % 2 = 1 := by rcases h_odd with ⟨k, rfl⟩; omega
    have _hm1 : m % 2 = 1 := by
      have h_m2 : (m : ZMod 2)^2 = 1 := by
        have h1 : (N : ZMod 2) = 1 := by rw [←ZMod.natCast_mod N 2, hn1]; rfl
        have h_cast : (N : ZMod 2) = (m : ZMod 2)^2 := by rw [hm_sq]; push_cast; rfl
        rw [←h_cast, h1]
      have h_decide2 : ∀ (x : ZMod 2), x^2 = 1 → x = 1 := by decide
      have h_m2_1 : (m : ZMod 2) = 1 := h_decide2 (m : ZMod 2) h_m2
      have h_m_val2 : (m : ZMod 2).val = (1 : ZMod 2).val := by rw [h_m2_1]
      rw [ZMod.val_natCast] at h_m_val2
      exact h_m_val2
    omega
  have h_sigma_zmod : (sigma N : ZMod 4) = 3 := by
    push_cast [h_sigma]
    exact h_decide (m : ZMod 4) h_m_val
  have h_val : (sigma N : ZMod 4).val = (3 : ZMod 4).val := by rw [h_sigma_zmod]
  rw [ZMod.val_natCast] at h_val
  exact h_val

theorem qpn_mod_5_eq_1 {N : ℕ} (h_qpn : IsQuasiperfect N) (h_5_dvd : 5 ∣ N) : sigma N % 5 = 1 := by
  have ⟨_, m, hm_sq⟩ := qpn_is_odd_square h_qpn
  have h_5_dvd_m : 5 ∣ m := Nat.Prime.dvd_of_dvd_pow (by decide) (by rw [← hm_sq]; exact h_5_dvd)
  have h_sigma : sigma N = 2 * m ^ 2 + 1 := by rw [h_qpn.2, hm_sq]
  have h_decide : ∀ (x : ZMod 5), x = 0 → 2 * x ^ 2 + 1 = 1 := by decide
  have h_m_zmod : (m : ZMod 5) = 0 := by
    rw [←ZMod.natCast_mod m 5, Nat.mod_eq_zero_of_dvd h_5_dvd_m]
    rfl
  have h_sigma_zmod : (sigma N : ZMod 5) = 1 := by
    push_cast [h_sigma]
    exact h_decide (m : ZMod 5) h_m_zmod
  have h_val : (sigma N : ZMod 5).val = (1 : ZMod 5).val := by rw [h_sigma_zmod]
  rw [ZMod.val_natCast] at h_val
  exact h_val

theorem qpn_mod_5_neq {N : ℕ} (h_qpn : IsQuasiperfect N) : sigma N % 5 ≠ 0 ∧ sigma N % 5 ≠ 2 := by
  have ⟨_, m, hm_sq⟩ := qpn_is_odd_square h_qpn
  have h_sigma : sigma N = 2 * m ^ 2 + 1 := by rw [h_qpn.2, hm_sq]
  have h_decide : ∀ (x : ZMod 5), 2 * x ^ 2 + 1 ≠ 0 ∧ 2 * x ^ 2 + 1 ≠ 2 := by decide
  have h_sigma_zmod := h_decide (m : ZMod 5)
  have h_sigma_eq : (sigma N : ZMod 5) = 2 * (m : ZMod 5) ^ 2 + 1 := by push_cast [h_sigma]; rfl
  rw [←h_sigma_eq] at h_sigma_zmod
  have h_neq_0 : sigma N % 5 ≠ 0 := by
    intro h_eq
    have h_zmod_0 : (sigma N : ZMod 5) = 0 := by rw [←ZMod.natCast_mod (sigma N) 5, h_eq]; rfl
    exact h_sigma_zmod.1 h_zmod_0
  have h_neq_2 : sigma N % 5 ≠ 2 := by
    intro h_eq
    have h_zmod_2 : (sigma N : ZMod 5) = 2 := by rw [←ZMod.natCast_mod (sigma N) 5, h_eq]; rfl
    exact h_sigma_zmod.2 h_zmod_2
  exact ⟨h_neq_0, h_neq_2⟩

theorem is_valid_mod_8 (q : ℕ) (hq : q.Prime) (hq_odd : q ≠ 2) (h_mod : q % 8 = 1 ∨ q % 8 = 3) :
    q % 8 ≠ 5 ∧ q % 8 ≠ 7 := by
  rcases h_mod with h1 | h3 <;> omega

end UALBF.QPN.Obstruction
