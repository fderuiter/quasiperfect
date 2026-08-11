import UALBF.Basic
import Mathlib.Data.ZMod.Basic
import Mathlib.Data.Nat.Factorization.Basic
import UALBF.FFI
import UALBF.QPN.TouchardQPN
import Mathlib.Tactic.Ring

namespace UALBF.Engine.TouchardBridge

open UALBF
open UALBF.QPN.TouchardQPN

theorem sigma_p_mod_2 (p e : ℕ) (hp : p.Prime) (hp1 : p % 2 = 1) (he : e % 2 = 1) :
  (sigma (p ^ e) : ZMod 2) = 0 := by
  have h_sum : sigma (p ^ e) = ∑ x ∈ Finset.range (e + 1), p ^ x := by
    exact (Nat.sum_divisors_prime_pow hp)
  rw [h_sum]
  push_cast
  have h_p_zmod : (p : ZMod 2) = 1 := by
    have h1 : ((p % 2 : ℕ) : ZMod 2) = (1 : ZMod 2) := by rw [hp1]; rfl
    have h2 : ((p % 2 : ℕ) : ZMod 2) = (p : ZMod 2) := by exact ZMod.natCast_mod p 2
    rw [←h2]
    exact h1
  have h_pow : ∀ x, (p : ZMod 2) ^ x = 1 := by
    intro x
    rw [h_p_zmod, one_pow]
  have h_sum_zmod : ∑ x ∈ Finset.range (e + 1), (p : ZMod 2) ^ x = ∑ x ∈ Finset.range (e + 1), (1 : ZMod 2) := by
    apply Finset.sum_congr rfl
    intro x _
    exact h_pow x
  rw [h_sum_zmod, Finset.sum_const, Finset.card_range, nsmul_eq_mul, mul_one]
  have h_e_zmod : (e : ZMod 2) = 1 := by
    have ha : ((e % 2 : ℕ) : ZMod 2) = (1 : ZMod 2) := by rw [he]; rfl
    have hb : ((e % 2 : ℕ) : ZMod 2) = (e : ZMod 2) := by exact ZMod.natCast_mod e 2
    rw [←hb]
    exact ha
  push_cast
  rw [h_e_zmod]
  rfl

theorem touchard_bridge (p e : ℕ) (hp : p.Prime)
  (h_p : p % 2 = 1) (h_e : e % 2 = 1) :
  sigma (p ^ e) % 2 = 0 := by
  have h1 := sigma_p_mod_2 p e hp h_p h_e
  have h3 : ((sigma (p ^ e)) : ZMod 2).val = (0 : ZMod 2).val := by rw [h1]
  rw [ZMod.val_natCast, ZMod.val_zero] at h3
  exact h3

theorem ualbf_check_touchard_soundness_ffi (p : UInt64) (two_e : UInt32) (hp : p.toNat.Prime)
  (h_ffi : UALBF.FFI.ualbf_check_touchard_impl p two_e = true) :
  sigma (p.toNat ^ (two_e.toNat)) % 2 = 0 := by
  unfold UALBF.FFI.ualbf_check_touchard_impl at h_ffi
  simp only [Bool.and_eq_true, beq_iff_eq] at h_ffi
  rcases h_ffi with ⟨h_e_eq, h_p_eq⟩
  have h_e : two_e.toNat % 2 = 1 := by
    have h_mod_eq : (two_e % 2).toNat = two_e.toNat % 2 := rfl
    have h_val : (two_e % 2).toNat = 1 := by rw [h_e_eq]; rfl
    rw [←h_mod_eq]
    exact h_val
  have h_p : p.toNat % 2 = 1 := by
    have h_mod_eq : (p % 2).toNat = p.toNat % 2 := rfl
    have h_val : (p % 2).toNat = 1 := by rw [h_p_eq]; rfl
    rw [←h_mod_eq]
    exact h_val
  exact touchard_bridge p.toNat two_e.toNat hp h_p h_e

lemma test_omega (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) :
  (w0.toNat + w1.toNat * 2^64 + w2.toNat * 2^128 + w3.toNat * 2^192 + w4.toNat * 2^256 + w5.toNat * 2^320 + w6.toNat * 2^384 + w7.toNat * 2^448) % 2^64 = w0.toNat := by
  have _h0 : w0.toNat < 2^64 := w0.toNat_lt
  have _h1 : w1.toNat < 2^64 := w1.toNat_lt
  have _h2 : w2.toNat < 2^64 := w2.toNat_lt
  have _h3 : w3.toNat < 2^64 := w3.toNat_lt
  have _h4 : w4.toNat < 2^64 := w4.toNat_lt
  have _h5 : w5.toNat < 2^64 := w5.toNat_lt
  have _h6 : w6.toNat < 2^64 := w6.toNat_lt
  have _h7 : w7.toNat < 2^64 := w7.toNat_lt
  have h2_64 : 2^64 = 18446744073709551616 := rfl
  have h2_128 : 2^128 = 340282366920938463463374607431768211456 := rfl
  have h2_192 : 2^192 = 6277101735386680763835789423207666416102355444464034512896 := rfl
  have h2_256 : 2^256 = 115792089237316195423570985008687907853269984665640564039457584007913129639936 := rfl
  have h2_320 : 2^320 = 2135987035920910082395021706169552114602704522356652769947041607822219725780640550022962086936576 := rfl
  have h2_384 : 2^384 = 39402006196394479212279040100143613805079739270465446667948293404245721771497210611414266254884915640806627990306816 := rfl
  have h2_448 : 2^448 = 726838724295606890549323807888004534353641360687318060281490199180639288113397923326191050713763565560762521606266177933534601628614656 := rfl
  rw [h2_64, h2_128, h2_192, h2_256, h2_320, h2_384, h2_448] at *
  omega

end UALBF.Engine.TouchardBridge
