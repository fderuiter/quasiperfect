import UALBF.Basic
import Mathlib.Data.ZMod.Basic
import Mathlib.Data.Nat.Factorization.Basic
import UALBF.FFI
import UALBF.Engine.SieveSoundness

namespace UALBF.Engine.Mod5Bridge

open UALBF
open UALBF.Engine.SieveSoundness

theorem sigma_p_mod_5 (p e : ℕ) (hp : p.Prime) (hp1 : p % 5 = 1) :
  (sigma (p ^ (2 * e)) : ZMod 5) = (2 * e + 1 : ℕ) := by
  have h_sum : sigma (p ^ (2 * e)) = ∑ x ∈ Finset.range (2 * e + 1), p ^ x := by
    exact (Nat.sum_divisors_prime_pow hp)
  rw [h_sum]
  push_cast
  have h_p_zmod : (p : ZMod 5) = 1 := by
    have h1 : ((p % 5 : ℕ) : ZMod 5) = (1 : ZMod 5) := by rw [hp1]; rfl
    have h2 : ((p % 5 : ℕ) : ZMod 5) = (p : ZMod 5) := by exact ZMod.natCast_mod p 5
    rw [←h2]
    exact h1
  have h_pow : ∀ x, (p : ZMod 5) ^ x = 1 := by
    intro x
    rw [h_p_zmod, one_pow]
  have h_sum_zmod : ∑ x ∈ Finset.range (2 * e + 1), (p : ZMod 5) ^ x = ∑ x ∈ Finset.range (2 * e + 1), (1 : ZMod 5) := by
    apply Finset.sum_congr rfl
    intro x _
    exact h_pow x
  rw [h_sum_zmod, Finset.sum_const, Finset.card_range, nsmul_eq_mul, mul_one]
  push_cast
  rfl

theorem mod5_bridge (p e : ℕ) (hp : p.Prime)
  (h_p : p % 5 = 1) (h_e : e % 5 = 2) :
  sigma (p ^ (2 * e)) % 5 = 0 := by
  have h1 := sigma_p_mod_5 p e hp h_p
  have h_e_zmod : (e : ZMod 5) = 2 := by
    have ha : ((e % 5 : ℕ) : ZMod 5) = (2 : ZMod 5) := by rw [h_e]; rfl
    have hb : ((e % 5 : ℕ) : ZMod 5) = (e : ZMod 5) := by exact ZMod.natCast_mod e 5
    rw [←hb]
    exact ha
  have h2 : ((2 * e + 1 : ℕ) : ZMod 5) = 0 := by
    push_cast
    rw [h_e_zmod]
    decide
  rw [h2] at h1
  have h3 : ((sigma (p ^ (2 * e))) : ZMod 5).val = (0 : ZMod 5).val := by rw [h1]
  rw [ZMod.val_natCast, ZMod.val_zero] at h3
  exact h3

theorem ualbf_check_mod_5_soundness_ffi (p : UInt64) (two_e : UInt32) (hp : p.toNat.Prime)
  (_h_even : two_e.toNat % 2 = 0)
  (h_ffi : UALBF.FFI.ualbf_check_mod_5_impl p two_e = true)
  {N : ℕ} (h_qpn : IsQuasiperfect N) :
  ¬ ExactValuation p.toNat (two_e.toNat) N := by
  have h_bad : sigma (p.toNat ^ (two_e.toNat)) % 5 = 0 := by
    unfold UALBF.FFI.ualbf_check_mod_5_impl at h_ffi
    simp only [Bool.and_eq_true, beq_iff_eq] at h_ffi
    rcases h_ffi with ⟨h_p_mod, h_e_mod⟩
    have h_p : p.toNat % 5 = 1 := by
      have h_mod_eq : (p % 5).toNat = p.toNat % 5 := rfl
      have h_val : (p % 5).toNat = 1 := by rw [h_p_mod]; rfl
      rw [←h_mod_eq]
      exact h_val
    have h_e : (two_e.toNat / 2) % 5 = 2 := by
      have h_mod_eq : ((two_e / 2) % 5).toNat = (two_e.toNat / 2) % 5 := rfl
      have h_val : ((two_e / 2) % 5).toNat = 2 := by rw [h_e_mod]; rfl
      rw [←h_mod_eq]
      exact h_val
    have h_pow_eq : two_e.toNat = 2 * (two_e.toNat / 2) := by omega
    rw [h_pow_eq]
    exact mod5_bridge p.toNat (two_e.toNat / 2) hp h_p h_e
  have h_pow_eq : two_e.toNat = 2 * (two_e.toNat / 2) := by omega
  rw [h_pow_eq] at h_bad
  rw [h_pow_eq]
  exact rust_sieve_soundness_mod_5 h_qpn hp h_bad

end UALBF.Engine.Mod5Bridge
