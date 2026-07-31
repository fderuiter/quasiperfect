import UALBF.Basic
import UALBF.QPN.BasicProperties
import Mathlib.Data.ZMod.Basic
import Mathlib.Tactic.Ring

/-!
# Touchard's Congruence for QPNs (Modulo 24 obstruction)

Proves that if N is a quasiperfect number (QPN), then
  σ(N) ≡ 3 or 19 (mod 24).
-/

namespace UALBF.QPN.TouchardQPN

open Finset Nat
open UALBF
open UALBF.QPN.BasicProperties

theorem qpn_sigma_mod_24 {N : ℕ} (h_qpn : IsQuasiperfect N) :
    sigma N % 24 = 3 ∨ sigma N % 24 = 19 := by
  have ⟨h_odd, m, hm_sq⟩ := qpn_is_odd_square h_qpn

  -- First, N % 2 = 1.
  have hn1 : N % 2 = 1 := by rcases h_odd with ⟨k, rfl⟩; omega

  -- m % 2 = 1.
  have hm1 : m % 2 = 1 := by
    have h_m2 : (m : ZMod 2)^2 = 1 := by
      have h1 : (N : ZMod 2) = 1 := by rw [←ZMod.natCast_mod N 2, hn1]; rfl
      have h_cast : (N : ZMod 2) = (m : ZMod 2)^2 := by rw [hm_sq]; push_cast; rfl
      rw [←h_cast, h1]
    have h_decide2 : ∀ (x : ZMod 2), x^2 = 1 → x = 1 := by decide
    have h_m2_1 : (m : ZMod 2) = 1 := h_decide2 (m : ZMod 2) h_m2
    have h_m_val2 : (m : ZMod 2).val = (1 : ZMod 2).val := by rw [h_m2_1]
    rw [ZMod.val_natCast] at h_m_val2
    exact h_m_val2

  -- N % 8 = 1.
  have h_N_mod_8 : (N : ZMod 8) = 1 := by
    rw [hm_sq]
    push_cast
    have h_decide : ∀ (x : ZMod 8), x.val % 2 = 1 → x ^ 2 = 1 := by decide
    apply h_decide
    rw [ZMod.val_natCast]
    omega

  -- N % 3 = 0 or 1.
  have h_N_mod_3 : (N : ZMod 3) = 0 ∨ (N : ZMod 3) = 1 := by
    rw [hm_sq]
    push_cast
    have h_decide : ∀ (x : ZMod 3), x ^ 2 = 0 ∨ x ^ 2 = 1 := by decide
    exact h_decide (m : ZMod 3)

  -- Now combine mod 8 and mod 3 into mod 24.
  -- We establish that N = 24 * (N / 24) + N % 24
  have h_div : N = 24 * (N / 24) + N % 24 := (Nat.div_add_mod N 24).symm

  have h_cast2 : (N : ZMod 8) = (((N % 24 : ℕ) : ZMod 8)) := by
    nth_rw 1 [h_div]
    push_cast
    have h24 : (24 : ZMod 8) = 0 := rfl
    rw [h24, zero_mul, zero_add]

  have h_cast3 : (N : ZMod 3) = (((N % 24 : ℕ) : ZMod 3)) := by
    nth_rw 1 [h_div]
    push_cast
    have h24 : (24 : ZMod 3) = 0 := rfl
    rw [h24, zero_mul, zero_add]

  have h_8_val : ((N : ZMod 24).val : ZMod 8) = 1 := by
    rw [ZMod.val_natCast]
    rw [←h_cast2]
    exact h_N_mod_8

  have h_3_val : ((N : ZMod 24).val : ZMod 3) = 0 ∨ ((N : ZMod 24).val : ZMod 3) = 1 := by
    rw [ZMod.val_natCast]
    rw [←h_cast3]
    exact h_N_mod_3

  have h_decide_24 : ∀ (x : ZMod 24), ((x.val : ZMod 8) = 1) ∧ (((x.val : ZMod 3) = 0) ∨ ((x.val : ZMod 3) = 1)) → x = 1 ∨ x = 9 := by decide

  have h_N_24 : (N : ZMod 24) = 1 ∨ (N : ZMod 24) = 9 := h_decide_24 (N : ZMod 24) ⟨h_8_val, h_3_val⟩

  -- Finally, σ(N) = 2*N + 1
  have h_sigma_zmod : (sigma N : ZMod 24) = 2 * (N : ZMod 24) + 1 := by
    push_cast [h_qpn.2]
    rfl

  have h_sigma_24 : (sigma N : ZMod 24) = 3 ∨ (sigma N : ZMod 24) = 19 := by
    rcases h_N_24 with h1 | h9
    · left
      rw [h_sigma_zmod, h1]
      rfl
    · right
      rw [h_sigma_zmod, h9]
      rfl

  have h_val : (sigma N : ZMod 24).val = 3 ∨ (sigma N : ZMod 24).val = 19 := by
    rcases h_sigma_24 with h3 | h19
    · left; rw [h3]; rfl
    · right; rw [h19]; rfl

  rw [ZMod.val_natCast] at h_val
  exact h_val

end UALBF.QPN.TouchardQPN
