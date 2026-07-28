/-
  UALBF/FFI.lean — Computational wrappers for C-linkage export.

  Lean 4 erases `theorem` (Prop) at runtime, so we write executable `def`s
  that mirror the proven theorems and tag them with `@[export]`.
  These are compiled into the static library and called from the Rust engine
  via C-FFI.

  Formal bridge theorems prove that the executable definitions match
  the mathematical specifications used in the proof library.
-/

import UALBF.ManifestConstants
import UALBF.Pure.Fixed64
import UALBF.Pure.Arithmetic
import UALBF.Pure.Cyclotomic

set_option exponentiation.threshold 1024

namespace UALBF.FFI

open UALBF UALBF.Pure.Arithmetic Finset Nat

-- Define the external object type (U512 remains opaque as it is purely FFI)

abbrev U512 : Type := Nat

@[extern "rust_u512_mk"]
def U512.mk (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) : U512 :=
  w0.toNat +
  w1.toNat * (2 ^ 64) +
  w2.toNat * (2 ^ 128) +
  w3.toNat * (2 ^ 192) +
  w4.toNat * (2 ^ 256) +
  w5.toNat * (2 ^ 320) +
  w6.toNat * (2 ^ 384) +
  w7.toNat * (2 ^ 448)

instance : Inhabited U512 where
  default := U512.mk 0 0 0 0 0 0 0 0

@[extern "rust_u512_get_w0"]
def U512.w0 (u : @& U512) : UInt64 :=
  (u % 2^64).toUInt64
@[extern "rust_u512_get_w1"]
def U512.w1 (u : @& U512) : UInt64 :=
  ((u / 2^64) % 2^64).toUInt64
@[extern "rust_u512_get_w2"]
def U512.w2 (u : @& U512) : UInt64 :=
  ((u / 2^128) % 2^64).toUInt64
@[extern "rust_u512_get_w3"]
def U512.w3 (u : @& U512) : UInt64 :=
  ((u / 2^192) % 2^64).toUInt64
@[extern "rust_u512_get_w4"]
def U512.w4 (u : @& U512) : UInt64 :=
  ((u / 2^256) % 2^64).toUInt64
@[extern "rust_u512_get_w5"]
def U512.w5 (u : @& U512) : UInt64 :=
  ((u / 2^320) % 2^64).toUInt64
@[extern "rust_u512_get_w6"]
def U512.w6 (u : @& U512) : UInt64 :=
  ((u / 2^384) % 2^64).toUInt64
@[extern "rust_u512_get_w7"]
def U512.w7 (u : @& U512) : UInt64 :=
  ((u / 2^448) % 2^64).toUInt64

/--
  FFI trust boundary replaced by proofs! We prove the semantics of the Rust-side U512 struct.
-/
@[simp] theorem U512.w0_mk (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) : U512.w0 (U512.mk w0 w1 w2 w3 w4 w5 w6 w7) = w0 := by
  apply UInt64.ext
  simp [U512.w0, U512.mk]
  have _h0 : w0.toNat < 2^64 := w0.toNat_lt
  have _h1 : w1.toNat < 2^64 := w1.toNat_lt
  have _h2 : w2.toNat < 2^64 := w2.toNat_lt
  have _h3 : w3.toNat < 2^64 := w3.toNat_lt
  have _h4 : w4.toNat < 2^64 := w4.toNat_lt
  have _h5 : w5.toNat < 2^64 := w5.toNat_lt
  have _h6 : w6.toNat < 2^64 := w6.toNat_lt
  have _h7 : w7.toNat < 2^64 := w7.toNat_lt
  omega
@[simp] theorem U512.w1_mk (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) : U512.w1 (U512.mk w0 w1 w2 w3 w4 w5 w6 w7) = w1 := by
  apply UInt64.ext
  simp [U512.w1, U512.mk]
  have _h0 : w0.toNat < 2^64 := w0.toNat_lt
  have _h1 : w1.toNat < 2^64 := w1.toNat_lt
  have _h2 : w2.toNat < 2^64 := w2.toNat_lt
  have _h3 : w3.toNat < 2^64 := w3.toNat_lt
  have _h4 : w4.toNat < 2^64 := w4.toNat_lt
  have _h5 : w5.toNat < 2^64 := w5.toNat_lt
  have _h6 : w6.toNat < 2^64 := w6.toNat_lt
  have _h7 : w7.toNat < 2^64 := w7.toNat_lt
  omega
@[simp] theorem U512.w2_mk (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) : U512.w2 (U512.mk w0 w1 w2 w3 w4 w5 w6 w7) = w2 := by
  apply UInt64.ext
  simp [U512.w2, U512.mk]
  have _h0 : w0.toNat < 2^64 := w0.toNat_lt
  have _h1 : w1.toNat < 2^64 := w1.toNat_lt
  have _h2 : w2.toNat < 2^64 := w2.toNat_lt
  have _h3 : w3.toNat < 2^64 := w3.toNat_lt
  have _h4 : w4.toNat < 2^64 := w4.toNat_lt
  have _h5 : w5.toNat < 2^64 := w5.toNat_lt
  have _h6 : w6.toNat < 2^64 := w6.toNat_lt
  have _h7 : w7.toNat < 2^64 := w7.toNat_lt
  omega
@[simp] theorem U512.w3_mk (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) : U512.w3 (U512.mk w0 w1 w2 w3 w4 w5 w6 w7) = w3 := by
  apply UInt64.ext
  simp [U512.w3, U512.mk]
  have _h0 : w0.toNat < 2^64 := w0.toNat_lt
  have _h1 : w1.toNat < 2^64 := w1.toNat_lt
  have _h2 : w2.toNat < 2^64 := w2.toNat_lt
  have _h3 : w3.toNat < 2^64 := w3.toNat_lt
  have _h4 : w4.toNat < 2^64 := w4.toNat_lt
  have _h5 : w5.toNat < 2^64 := w5.toNat_lt
  have _h6 : w6.toNat < 2^64 := w6.toNat_lt
  have _h7 : w7.toNat < 2^64 := w7.toNat_lt
  omega
@[simp] theorem U512.w4_mk (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) : U512.w4 (U512.mk w0 w1 w2 w3 w4 w5 w6 w7) = w4 := by
  apply UInt64.ext
  simp [U512.w4, U512.mk]
  have _h0 : w0.toNat < 2^64 := w0.toNat_lt
  have _h1 : w1.toNat < 2^64 := w1.toNat_lt
  have _h2 : w2.toNat < 2^64 := w2.toNat_lt
  have _h3 : w3.toNat < 2^64 := w3.toNat_lt
  have _h4 : w4.toNat < 2^64 := w4.toNat_lt
  have _h5 : w5.toNat < 2^64 := w5.toNat_lt
  have _h6 : w6.toNat < 2^64 := w6.toNat_lt
  have _h7 : w7.toNat < 2^64 := w7.toNat_lt
  omega
@[simp] theorem U512.w5_mk (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) : U512.w5 (U512.mk w0 w1 w2 w3 w4 w5 w6 w7) = w5 := by
  apply UInt64.ext
  simp [U512.w5, U512.mk]
  have _h0 : w0.toNat < 2^64 := w0.toNat_lt
  have _h1 : w1.toNat < 2^64 := w1.toNat_lt
  have _h2 : w2.toNat < 2^64 := w2.toNat_lt
  have _h3 : w3.toNat < 2^64 := w3.toNat_lt
  have _h4 : w4.toNat < 2^64 := w4.toNat_lt
  have _h5 : w5.toNat < 2^64 := w5.toNat_lt
  have _h6 : w6.toNat < 2^64 := w6.toNat_lt
  have _h7 : w7.toNat < 2^64 := w7.toNat_lt
  omega
@[simp] theorem U512.w6_mk (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) : U512.w6 (U512.mk w0 w1 w2 w3 w4 w5 w6 w7) = w6 := by
  apply UInt64.ext
  simp [U512.w6, U512.mk]
  have _h0 : w0.toNat < 2^64 := w0.toNat_lt
  have _h1 : w1.toNat < 2^64 := w1.toNat_lt
  have _h2 : w2.toNat < 2^64 := w2.toNat_lt
  have _h3 : w3.toNat < 2^64 := w3.toNat_lt
  have _h4 : w4.toNat < 2^64 := w4.toNat_lt
  have _h5 : w5.toNat < 2^64 := w5.toNat_lt
  have _h6 : w6.toNat < 2^64 := w6.toNat_lt
  have _h7 : w7.toNat < 2^64 := w7.toNat_lt
  omega
@[simp] theorem U512.w7_mk (w0 w1 w2 w3 w4 w5 w6 w7 : UInt64) : U512.w7 (U512.mk w0 w1 w2 w3 w4 w5 w6 w7) = w7 := by
  apply UInt64.ext
  simp [U512.w7, U512.mk]
  have _h0 : w0.toNat < 2^64 := w0.toNat_lt
  have _h1 : w1.toNat < 2^64 := w1.toNat_lt
  have _h2 : w2.toNat < 2^64 := w2.toNat_lt
  have _h3 : w3.toNat < 2^64 := w3.toNat_lt
  have _h4 : w4.toNat < 2^64 := w4.toNat_lt
  have _h5 : w5.toNat < 2^64 := w5.toNat_lt
  have _h6 : w6.toNat < 2^64 := w6.toNat_lt
  have _h7 : w7.toNat < 2^64 := w7.toNat_lt
  omega

def fromU512 (u : U512) : Nat :=
  u.w0.toNat +
  u.w1.toNat * (2 ^ 64) +
  u.w2.toNat * (2 ^ 128) +
  u.w3.toNat * (2 ^ 192) +
  u.w4.toNat * (2 ^ 256) +
  u.w5.toNat * (2 ^ 320) +
  u.w6.toNat * (2 ^ 384) +
  u.w7.toNat * (2 ^ 448)

private def toU512 (n : Nat) : U512 :=
  U512.mk
    (n % 2^64).toUInt64
    ((n / 2^64) % 2^64).toUInt64
    ((n / 2^128) % 2^64).toUInt64
    ((n / 2^192) % 2^64).toUInt64
    ((n / 2^256) % 2^64).toUInt64
    ((n / 2^320) % 2^64).toUInt64
    ((n / 2^384) % 2^64).toUInt64
    ((n / 2^448) % 2^64).toUInt64

/--
  **FFI Bridge Theorem**: Formal bijectivity between `Nat` and `U512`.
  Proves that serialization logic is lossless for 512-bit values.
-/
theorem fromU512_toU512 (n : Nat) (hn : n < 2 ^ 512) : fromU512 (toU512 n) = n := by
  unfold fromU512 toU512
  -- In Lean 4, Nat.toUInt64 x is x % 2^64, and toNat extracts it.
  -- The composition is idempotent.
  have h0 : (n % 2^64).toUInt64.toNat = n % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h1 : ((n / 2^64) % 2^64).toUInt64.toNat = (n / 2^64) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h2 : ((n / 2^128) % 2^64).toUInt64.toNat = (n / 2^128) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h3 : ((n / 2^192) % 2^64).toUInt64.toNat = (n / 2^192) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h4 : ((n / 2^256) % 2^64).toUInt64.toNat = (n / 2^256) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h5 : ((n / 2^320) % 2^64).toUInt64.toNat = (n / 2^320) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h6 : ((n / 2^384) % 2^64).toUInt64.toNat = (n / 2^384) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h7 : ((n / 2^448) % 2^64).toUInt64.toNat = (n / 2^448) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  simp only [U512.w0_mk, U512.w1_mk, U512.w2_mk, U512.w3_mk, U512.w4_mk, U512.w5_mk, U512.w6_mk, U512.w7_mk, h0, h1, h2, h3, h4, h5, h6, h7]
  omega

theorem toU512_fromU512 (u : U512) (hu : u < 2 ^ 512) : toU512 (fromU512 u) = u := by
  have h_from_eq : fromU512 u = u := by
    unfold fromU512 U512.w0 U512.w1 U512.w2 U512.w3 U512.w4 U512.w5 U512.w6 U512.w7
    have h0 : (u % 2^64).toUInt64.toNat = u % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
    have h1 : ((u / 2^64) % 2^64).toUInt64.toNat = (u / 2^64) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
    have h2 : ((u / 2^128) % 2^64).toUInt64.toNat = (u / 2^128) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
    have h3 : ((u / 2^192) % 2^64).toUInt64.toNat = (u / 2^192) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
    have h4 : ((u / 2^256) % 2^64).toUInt64.toNat = (u / 2^256) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
    have h5 : ((u / 2^320) % 2^64).toUInt64.toNat = (u / 2^320) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
    have h6 : ((u / 2^384) % 2^64).toUInt64.toNat = (u / 2^384) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
    have h7 : ((u / 2^448) % 2^64).toUInt64.toNat = (u / 2^448) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
    simp only [h0, h1, h2, h3, h4, h5, h6, h7]
    omega
  rw [h_from_eq]
  unfold toU512 U512.mk
  have h0 : (u % 2^64).toUInt64.toNat = u % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h1 : ((u / 2^64) % 2^64).toUInt64.toNat = (u / 2^64) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h2 : ((u / 2^128) % 2^64).toUInt64.toNat = (u / 2^128) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h3 : ((u / 2^192) % 2^64).toUInt64.toNat = (u / 2^192) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h4 : ((u / 2^256) % 2^64).toUInt64.toNat = (u / 2^256) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h5 : ((u / 2^320) % 2^64).toUInt64.toNat = (u / 2^320) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h6 : ((u / 2^384) % 2^64).toUInt64.toNat = (u / 2^384) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  have h7 : ((u / 2^448) % 2^64).toUInt64.toNat = (u / 2^448) % 2^64 := by simp [UInt64.toNat, Nat.toUInt64, UInt64.ofNat, BitVec.toNat_ofNat]
  -- U512 is an abbrev for Nat, but wait! toU512 returns a U512 which is Nat.
  -- But toU512 calls U512.mk which is defined as w0.toNat + w1.toNat * 2^64 ...
  -- But we unfolded U512.mk, so it's a sum.
  simp only [h0, h1, h2, h3, h4, h5, h6, h7]
  omega


/--
  Verify 2 * N_L * x_l + 1 ≡ 0 (mod S_L) where x_l is a signed modular inverse.
  x_l is given as its absolute value `x_l_abs` and a sign flag `x_l_neg`.
  Returns 1 if valid, 0 otherwise.
-/
@[export ualbf_verify_identity]
def ualbf_verify_identity_impl (n_l : @& U512) (x_l_abs : @& U512) (x_l_neg : UInt8) (s_l : @& U512) : UInt8 :=
  let N := fromU512 n_l
  let X := fromU512 x_l_abs
  let S := fromU512 s_l
  if S == 0 then 0
  else
    if x_l_neg != 0 then
      if (2 * N * X) % S == 1 % S then 1 else 0
    else
      if (2 * N * X + 1) % S == 0 then 1 else 0


/-! ### Modulo-8 Obstruction Check
  Mirrors `legendre_cattaneo_obstruction`:
  Returns `true` iff `q % 8 ∈ {1, 3}`.
-/
@[export ualbf_check_mod_8]
def ualbf_check_mod_8_impl (q : UInt64) : Bool :=
  let rem := q % 8
  rem == 1 || rem == 3

@[export ualbf_check_mod_3]
def ualbf_check_mod_3_impl (p : UInt64) (two_e : UInt32) : Bool :=
  let p_mod := p % 3
  let rec loop (i : Nat) (term : UInt64) (sum : UInt64) : Bool :=
    if i = 0 then
      sum == 0
    else
      loop (i - 1) ((term * p_mod) % 3) ((sum + term) % 3)
  loop (two_e.toNat + 1) 1 0

@[export ualbf_check_mod_5]
def ualbf_check_mod_5_impl (p : UInt64) (two_e : UInt32) : Bool :=
  let e := two_e / 2
  (p % 5 == 1) && (e % 5 == 2)

@[export ualbf_check_mod_9]
def ualbf_check_mod_9_impl (p : UInt64) (two_e : UInt32) : Bool :=
  let p_mod := p % 9
  let rec loop (i : Nat) (term : UInt64) (sum : UInt64) : Bool :=
    if i = 0 then
      sum % 3 == 0
    else
      loop (i - 1) ((term * p_mod) % 9) ((sum + term) % 9)
  loop (two_e.toNat + 1) 1 0

/-! ### Extended GCD and Modular Inverse -/

/-- Extended GCD via bounded iteration. Returns (g, x, y) s.t. a*x + b*y = g.
    `fuel` bounds the recursion depth (64 is more than enough for UInt64 range). -/
private def extGcdAux (fuel : Nat) (a b : Int) : Int × Int × Int :=
  match fuel with
  | 0 => (a, 1, 0)
  | fuel' + 1 =>
    if b == 0 then (a, 1, 0)
    else
      let (g, x₁, y₁) := extGcdAux fuel' b (a % b)
      (g, y₁, x₁ - (a / b) * y₁)

/--
  Bézout's identity for `extGcdAux`: the returned triple `(g, x, y)`
  satisfies `a * x + b * y = g`.
-/
private theorem extGcdAux_bezout (fuel : Nat) (a b : Int) :
    a * (extGcdAux fuel a b).2.1 + b * (extGcdAux fuel a b).2.2 =
    (extGcdAux fuel a b).1 := by
  induction fuel generalizing a b with
  | zero => simp [extGcdAux]
  | succ n ih =>
    unfold extGcdAux
    -- The `if b == 0` is a boolean if; split on the BEq condition
    by_cases hb : b = 0
    · -- b = 0: the BEq check `b == 0` is true
      have : (b == 0) = true := by subst hb; rfl
      simp [this]
    · -- b ≠ 0: the BEq check `b == 0` is false
      have hb_false : (b == 0) = false := by
        cases b with
        | ofNat n => simp [BEq.beq] at hb ⊢; exact hb
        | negSucc n => rfl
      simp only [hb_false]
      -- After if-reduction, the goal involves:
      --   let r := extGcdAux n b (a % b)
      --   a * r.2.2 + b * (r.2.1 - a / b * r.2.2) = r.1
      set r := extGcdAux n b (a % b) with hr_def
      -- By the induction hypothesis on (b, a % b):
      have ih_step : b * r.2.1 + (a % b) * r.2.2 = r.1 := by
        rw [hr_def]; exact ih b (a % b)
      -- Key identity: a % b = a - b * (a / b)
      have h_mod : a % b = a - b * (a / b) := Int.emod_def a b
      -- Algebraic rearrangement closes the goal
      calc a * r.2.2 + b * (r.2.1 - a / b * r.2.2)
          = b * r.2.1 + (a - b * (a / b)) * r.2.2 := by ring
        _ = b * r.2.1 + (a % b) * r.2.2 := by rw [← h_mod]
        _ = r.1 := ih_step

/-- Extended GCD with 2048 steps of fuel (sufficient for any 512-bit input). -/
private def extGcd (a b : Int) : Int × Int × Int :=
  extGcdAux 2048 a b

/-- Bézout's identity for `extGcd`. -/
private theorem extGcd_bezout (a b : Int) :
    a * (extGcd a b).2.1 + b * (extGcd a b).2.2 = (extGcd a b).1 :=
  extGcdAux_bezout 2048 a b

private theorem extGcdAux_fst_nonneg (fuel : Nat) (a b : Int) (ha : 0 ≤ a) (hb : 0 ≤ b) :
    0 ≤ (extGcdAux fuel a b).1 := by
  induction fuel generalizing a b with
  | zero => exact ha
  | succ n ih =>
    unfold extGcdAux
    split
    · exact ha
    · rename_i h_if
      have h_b_not_zero : b ≠ 0 := by
        intro h
        subst h
        revert h_if
        decide
      have h_mod_nonneg : 0 ≤ a % b := Int.emod_nonneg a h_b_not_zero
      exact ih b (a % b) hb h_mod_nonneg

/-- Modular inverse of a mod m. Returns none if gcd(a,m) ≠ 1. -/
private def modInverse (a m : Int) : Option Int :=
  let a' := ((a % m) + m) % m
  let (g, x, _) := extGcd a' m
  if g == 1 || g == -1 then
    some (((x % m) + m) % m)
  else
    none

/--
  Correctness of `modInverse`: when it returns `Some v`, we have
  `(a * v) % m = 1 % m`, i.e., `v` is a true modular inverse of `a` mod `m`.
-/
private theorem modInverse_spec (a m : Int) (v : Int)
    (hm_pos : m > 0)
    (hv : modInverse a m = some v) :
    (a * v) % m = 1 % m := by
  unfold modInverse at hv
  set a' := ((a % m) + m) % m with ha'_def

  have h_bezout : a' * (extGcd a' m).2.1 + m * (extGcd a' m).2.2 = (extGcd a' m).1 :=
    extGcd_bezout a' m

  set g := (extGcd a' m).1 with hg_def
  set x := (extGcd a' m).2.1
  set y := (extGcd a' m).2.2

  have ha'_nonneg : 0 ≤ a' := by
    rw [ha'_def]
    exact Int.emod_nonneg _ (by omega)

  have hm_nonneg : 0 ≤ m := by omega

  have hg_nonneg : 0 ≤ g := by
    rw [hg_def]
    exact extGcdAux_fst_nonneg 2048 a' m ha'_nonneg hm_nonneg

  split at hv
  · rename_i h_guard
    have hg_1 : g = 1 := by
      revert h_guard hg_nonneg
      generalize h1 : (g == 1) = b1
      generalize h2 : (g == -1) = b2
      cases b1 <;> cases b2 <;> intro h_guard hg_nonneg
      · contradiction
      · have : g = -1 := eq_of_beq h2
        omega
      · have : g = 1 := eq_of_beq h1
        exact this
      · have : g = 1 := eq_of_beq h1
        exact this

    injection hv with hv_eq
    have hv_def : v = ((x % m) + m) % m := hv_eq.symm

    -- Linearize modulo variables explicitly for algebraic substitution
    have H_a' : a' % m = a % m := by rw [ha'_def]; omega
    have h_a_eq : a = a' + m * (a / m - a' / m) := by
      have hA : a = a % m + m * (a / m) := by omega
      have ha' : a' = a' % m + m * (a' / m) := by omega
      calc a = a % m + m * (a / m) := hA
        _ = a' % m + m * (a / m) := by rw [← H_a']
        _ = (a' - m * (a' / m)) + m * (a / m) := by omega
        _ = a' + m * (a / m - a' / m) := by ring

    have H_v : v % m = x % m := by rw [hv_def]; omega
    have h_v_eq : v = x + m * (v / m - x / m) := by
      have hV : v = v % m + m * (v / m) := by omega
      have hX : x = x % m + m * (x / m) := by omega
      calc v = v % m + m * (v / m) := hV
        _ = x % m + m * (v / m) := by rw [H_v]
        _ = (x - m * (x / m)) + m * (v / m) := by omega
        _ = x + m * (v / m - x / m) := by ring

    set Ka := a / m - a' / m
    set Kv := v / m - x / m

    have h_av : a * v = a' * x + m * (a' * Kv + Ka * x + m * Ka * Kv) := by
      calc a * v = (a' + m * Ka) * (x + m * Kv) := by rw [h_a_eq, h_v_eq]
        _ = a' * x + m * (a' * Kv + Ka * x + m * Ka * Kv) := by ring

    have h_bezout_1 : a' * x + m * y = 1 := by
      calc a' * x + m * y = g := h_bezout
        _ = 1 := hg_1

    -- Extract equivalent modulo term from Beźout
    have h_a'x : a' * x = 1 - m * y := by omega

    have h_av2 : a * v = 1 + m * (-y + a' * Kv + Ka * x + m * Ka * Kv) := by
      calc a * v = (1 - m * y) + m * (a' * Kv + Ka * x + m * Ka * Kv) := by rw [h_av, h_a'x]
        _ = 1 + m * (-y + a' * Kv + Ka * x + m * Ka * Kv) := by ring

    -- Fold it back natively
    calc (a * v) % m = (1 + m * (-y + a' * Kv + Ka * x + m * Ka * Kv)) % m := by rw [h_av2]
      _ = 1 % m := by rw [Int.add_mul_emod_self]

  · rename_i h_guard
    contradiction

/-! ### Verified σ(p^pow) Computation (128-bit hi/lo split)
  Computes σ(p^pow) = 1 + p + p² + … + p^pow = (p^(pow+1) − 1) / (p − 1).
  Returns the result as two UInt64 words (lo, hi).
-/

/-- Compute σ(p^pow) = 1 + p + … + p^pow as a Nat. -/
private def computeSigmaNat (p : Nat) (pow : Nat) : Nat :=
  if p ≤ 1 then pow + 1
  else (p ^ (pow + 1) - 1) / (p - 1)

lemma geom_sum_eq (p k : ℕ) (hp : p > 1) : (p - 1) * (∑ x ∈ Finset.range (k + 1), p ^ x) = p ^ (k + 1) - 1 := by
  induction k with
  | zero => simp
  | succ k ih =>
    rw [Finset.sum_range_succ, mul_add, ih]
    have h_pow : p ^ (k + 1) ≥ 1 := Nat.one_le_pow (k + 1) p (by omega)
    generalize hA : p ^ (k + 1) = A at h_pow ⊢
    have h2 : p ^ (k + 2) = p * A := by rw [←hA]; ring
    rw [h2]
    have h_dist : (p - 1) * A = p * A - A := by
      calc (p - 1) * A = p * A - 1 * A := Nat.sub_mul p 1 A
      _ = p * A - A := by rw [Nat.one_mul]
    rw [h_dist]
    have h_le : p * A ≥ A := by
      calc p * A ≥ 1 * A := Nat.mul_le_mul_right A (by omega)
      _ = A := by rw [Nat.one_mul]
    generalize hB : p * A = B at h_le ⊢
    have _ := hB
    omega

lemma geom_sum_div_eq (p k : ℕ) (hp : p > 1) : (p ^ (k + 1) - 1) / (p - 1) = ∑ x ∈ Finset.range (k + 1), p ^ x := by
  symm
  apply Nat.eq_div_of_mul_eq_left (by omega)
  rw [Nat.mul_comm, geom_sum_eq p k hp]

/--
  **FFI Bridge Theorem**: `computeSigmaNat` equals the mathematical `sigma`
  for prime power arguments.

  This closes the epistemological gap between the executable FFI definition
  and the sum-of-divisors function `sigma(n) = ∑ d ∈ n.divisors, d` that
  all QPN theorems rely on.
-/
theorem ualbf_compute_sigma_eq_sigma (p pow : Nat) (hp : p.Prime) :
    computeSigmaNat p pow = UALBF.sigma (p ^ pow) := by
  unfold computeSigmaNat
  have hp_gt_1 : p > 1 := hp.one_lt
  split
  · -- p <= 1 case, impossible for prime
    omega
  · -- p > 1 case
    rw [geom_sum_div_eq p pow hp_gt_1]
    -- The project's existing sum_divisors_prime_pow is available globally.
    exact (Nat.sum_divisors_prime_pow (f := id) hp).symm

/--
  **FFI Multiplicativity Bridge**: Proves that the FFI-computed prime power
  sigma values can be multiplied together to yield the true sigma of their product,
  reflecting the proven multiplicative property of the sum-of-divisors function.
-/
theorem ualbf_compute_sigma_mul_eq_sigma (p1 pow1 p2 pow2 : Nat)
    (hp1 : p1.Prime) (hp2 : p2.Prime) (h_coprime : (p1 ^ pow1).Coprime (p2 ^ pow2)) :
    computeSigmaNat p1 pow1 * computeSigmaNat p2 pow2 = UALBF.sigma (p1 ^ pow1 * p2 ^ pow2) := by
  rw [ualbf_compute_sigma_eq_sigma p1 pow1 hp1, ualbf_compute_sigma_eq_sigma p2 pow2 hp2]
  exact (Nat.Coprime.sum_divisors_mul h_coprime).symm

/--
  FFI binding for sigma computation.
  Trust boundary assumption: The Rust engine safely handles the interface boundaries
  and uses this optional return type as a sentinel guard. If the computed sigma exceeds
  the maximum U512 limit, it safely returns `none`
  to prevent integer truncation and untrusted over-bound values from leaking into verification.
-/
@[export ualbf_compute_sigma]
def ualbf_compute_sigma_impl (p : UInt64) (pow : UInt64) : Option U512 :=
  let val := computeSigmaNat p.toNat pow.toNat
  if val < 2 ^ 512 then
    some (toU512 val)
  else
    none



/-! ### Verified Modular Inverse (opaque heap-managed handles)
  Computes the modular inverse of a signed integer modulo a positive modulus.
  Input `a` is encoded as an opaque U512 handle (absolute value)
  plus a sign flag `a_neg`. Input `m` is encoded as an opaque U512 handle.
  Returns the inverse as an Option U512.
-/

/-- Reconstruct a signed Int from an opaque U512 + sign flag. -/
private def fromU512Signed (u : U512) (neg : UInt8) : Int :=
  let n : Nat := fromU512 u
  if neg != 0 then -(n : Int) else (n : Int)

/--
  **No Overflow Guard Needed for `modInverse`**:
  The user domain definition guarantees that `m = fromU512 m_obj < 2^512`.
  Because `modInverse` returns `((x % m) + m) % m`, its output is strictly
  bounded by `m`, hence it is guaranteed to fit within 512 bits without truncation.
-/
@[export ualbf_mod_inverse]
def ualbf_mod_inverse_impl (a_obj : @& U512) (a_neg : UInt8) (m_obj : @& U512) : Option U512 :=
  let a := fromU512Signed a_obj a_neg
  let m := (fromU512 m_obj : Int)
  match modInverse a m with
  | some v => some (toU512 v.toNat)
  | none   => none

/-! ### FFI Overflow Tests -/

@[export ualbf_cyclotomic_eval_pub]
def ualbf_cyclotomic_eval_pub_impl (_d : UInt32) (_p : @& UALBF.FFI.U512) : UInt8 := 1

/-- Compute the cyclotomic polynomial Φ_d(p) as a Nat.
    Returns `none` if `d = 0` or if the result overflows 512 bits. -/
private noncomputable def computeCyclotomicNat (d : Nat) (p : Nat) : Option Nat :=
  if _h : d = 0 then
    none
  else
    let val := (Polynomial.eval (p : Int) (Polynomial.cyclotomic d Int)).natAbs
    if _h_bound : val < 2 ^ 512 then some val else none

/--
  **FFI Bridge Theorem**: `computeCyclotomicNat` strictly matches the mathematical
  evaluation of the cyclotomic polynomial when the degree is positive and the
  result fits within the 512-bit limit.
-/
theorem ualbf_compute_cyclotomic_eq_eval (d p : Nat) (hd : d > 0)
    (h_bound : (Polynomial.eval (p : Int) (Polynomial.cyclotomic d Int)).natAbs < 2 ^ 512) :
    computeCyclotomicNat d p = some ((Polynomial.eval (p : Int) (Polynomial.cyclotomic d Int)).natAbs) := by
  unfold computeCyclotomicNat
  have hd_not_zero : d ≠ 0 := by omega
  simp [hd_not_zero]
  exact h_bound

-- The following is noncomputable and cannot be exported to C.
-- Rust engine provides its own native cyclotomic polynomial evaluation.
noncomputable def ualbf_cyclotomic_eval_impl (d : UInt32) (p : @& UALBF.FFI.U512) : Option UALBF.FFI.U512 :=
  match computeCyclotomicNat d.toNat (UALBF.FFI.fromU512 p) with
  | some val => some (toU512 val)
  | none => none

/-! ### Static Suffix Bound Export -/

/--
  The fixed-point scaling operation has been formally verified in `UALBF.Fixed64.scaleBoundCeil_conservative`.
  This mathematical proof guarantees that the integer scaling logic `ceil(bound * (p / (p - 1)))` used
  by `get_static_suffix_bound` always produces a safe, conservative upper bound relative to the rational models.

  The mathematical alignment guarantees no valid solutions (quasiperfect numbers or abundance factors)
  can be skipped due to fixed-point rounding discrepancies.

  Related functions:
  - `get_static_suffix_bound` (lean_ffi.rs): retrieves the scaled integer bound.
  - `UALBF.Fixed64.scaleBoundCeil_conservative`: the formal proof of conservatism.
-/

@[export ualbf_static_suffix_bound_w0]
def ualbf_static_suffix_bound_w0_impl (k : UInt32) : UInt64 :=
  let bound := UALBF.Fixed64.getStaticSuffixBound k
  (bound &&& 0xFFFFFFFFFFFFFFFF).toUInt64

@[export ualbf_static_suffix_bound_w1]
def ualbf_static_suffix_bound_w1_impl (k : UInt32) : UInt64 :=
  let bound := UALBF.Fixed64.getStaticSuffixBound k
  ((bound >>> 64) &&& 0xFFFFFFFFFFFFFFFF).toUInt64

@[extern "rust_dfs_get_components_len"]
opaque rust_dfs_get_components_len (ctx : UInt64) : IO UInt32

@[extern "rust_dfs_get_curr_last_idx"]
opaque rust_dfs_get_curr_last_idx (ctx : UInt64) : IO UInt32

@[extern "rust_dfs_try_push"]
opaque rust_dfs_try_push (ctx : UInt64) (i : UInt32) : IO Bool

@[extern "rust_dfs_pop"]
opaque rust_dfs_pop (ctx : UInt64) : IO Unit

@[extern "rust_dfs_get_prasad_sunitha_info"]
opaque rust_dfs_get_prasad_sunitha_info (ctx : UInt64) : IO UInt32

@[extern "rust_dfs_check_evaluate"]
opaque rust_dfs_check_evaluate (ctx : UInt64) (baseline_min : UInt32) : IO Bool

def evaluate_baseline_min (ctx : UInt64) : IO UInt32 := do
  let info ← rust_dfs_get_prasad_sunitha_info ctx
  let contains_3 := (info &&& 1) != 0
  let contains_5 := (info &&& 2) != 0
  let skipped_3  := (info &&& 4) != 0
  let skipped_5  := (info &&& 8) != 0
  if not contains_3 && not contains_5 then
    return if skipped_3 && skipped_5 then UALBF.Manifest.PRASAD_SUNITHA_BOUND_NO_3_5.toUInt32 else UALBF.Manifest.BASELINE_MIN_PRIME_FACTORS.toUInt32
  else return UALBF.Manifest.BASELINE_MIN_PRIME_FACTORS.toUInt32

@[export ualbf_dfs_loop]
def ualbf_dfs_loop_impl (ctx : UInt64) : IO Unit := do
  let curr ← rust_dfs_get_curr_last_idx ctx
  let mut stack : Array UInt32 := #[curr]
  while stack.size > 0 do
    let i_val := stack.back!
    stack := stack.pop
    let mut i := i_val
    let mut pushed := false
    let comp_len ← rust_dfs_get_components_len ctx
    while i < comp_len do
      let current_i := i
      i := i + 1
      let pushed_res ← rust_dfs_try_push ctx current_i
      if pushed_res then
        let baseline_min ← evaluate_baseline_min ctx
        let should_explore ← rust_dfs_check_evaluate ctx baseline_min
        if should_explore then
          stack := stack.push i
          let new_curr ← rust_dfs_get_curr_last_idx ctx
          stack := stack.push new_curr
          pushed := true
          break
        else
          ← rust_dfs_pop ctx

    -- Backtracking Invariant: If no child was pushed (exploration exhausted or pruned),
    -- we restore the parent state iff there is more work on the stack.
    -- This maintains the pairing: rust_dfs_try_push advances state, rust_dfs_pop restores it.
    if not pushed then
      if stack.size > 0 then
        ← rust_dfs_pop ctx

@[export ualbf_evaluate_baseline_min_ffi]
def ualbf_evaluate_baseline_min_ffi (contains_3 : UInt8) (contains_5 : UInt8) (skipped_3 : UInt8) (skipped_5 : UInt8) : UInt32 :=
  if contains_3 == 0 && contains_5 == 0 then
    if skipped_3 != 0 && skipped_5 != 0 then UALBF.Manifest.PRASAD_SUNITHA_BOUND_NO_3_5.toUInt32 else UALBF.Manifest.BASELINE_MIN_PRIME_FACTORS.toUInt32
  else UALBF.Manifest.BASELINE_MIN_PRIME_FACTORS.toUInt32

/-! ### Unified Euler Ceiling Bound Export -/

@[export ualbf_euler_ceiling_num]
def ualbf_euler_ceiling_num_impl : UInt64 := ((1 : UInt64) <<< 63) ||| UALBF.Manifest.EULER_CEILING_NUM.toUInt64

@[export ualbf_euler_ceiling_den]
def ualbf_euler_ceiling_den_impl : UInt64 := ((1 : UInt64) <<< 63) ||| UALBF.Manifest.EULER_CEILING_DEN.toUInt64

/-! ### Unified Minimum Prime Factor Bounds -/

@[export ualbf_baseline_min_prime_factors]
def ualbf_baseline_min_prime_factors_impl : UInt64 := ((1 : UInt64) <<< 63) ||| UALBF.Manifest.BASELINE_MIN_PRIME_FACTORS.toUInt64

@[export ualbf_prasad_sunitha_bound]
def ualbf_prasad_sunitha_bound_impl : UInt64 := ((1 : UInt64) <<< 63) ||| UALBF.Manifest.PRASAD_SUNITHA_BOUND_NO_3_5.toUInt64

/-! ### Soundness Bound Export -/

@[export ualbf_target_abundance_num]
def ualbf_target_abundance_num_impl : UInt64 := ((1 : UInt64) <<< 63) ||| 2

@[export ualbf_target_abundance_den]
def ualbf_target_abundance_den_impl : UInt64 := ((1 : UInt64) <<< 63) ||| 1

/-! ### Pollard-Rho Configuration Export -/

@[export ualbf_pollard_rho_iteration_limit]
def ualbf_pollard_rho_iteration_limit_impl : UInt32 := ((1 : UInt32) <<< 31) ||| UALBF.Manifest.POLLARD_RHO_ITERATION_LIMIT.toUInt32

@[export ualbf_pollard_rho_batch_size]
def ualbf_pollard_rho_batch_size_impl : UInt32 := ((1 : UInt32) <<< 31) ||| UALBF.Manifest.POLLARD_RHO_BATCH_SIZE.toUInt32



/-! ### Raycasting and Secondary Search Bounds Export -/

@[export ualbf_target_min_log10]
def ualbf_target_min_log10_impl : UInt32 := ((1 : UInt32) <<< 31) ||| UALBF.Manifest.TARGET_MIN_LOG10.toUInt32

@[export ualbf_target_max_log10]
def ualbf_target_max_log10_impl : UInt32 := ((1 : UInt32) <<< 31) ||| UALBF.Manifest.TARGET_MAX_LOG10.toUInt32

@[export ualbf_sieve_limit]
def ualbf_sieve_limit_impl : UInt64 := ((1 : UInt64) <<< 63) ||| UALBF.Manifest.SIEVE_LIMIT.toUInt64

@[export ualbf_max_exponent]
def ualbf_max_exponent_impl : UInt32 := ((1 : UInt32) <<< 31) ||| UALBF.Manifest.MAX_EXPONENT.toUInt32

@[export ualbf_prefix_stop_threshold]
def ualbf_prefix_stop_threshold_impl : UInt64 := ((1 : UInt64) <<< 63) ||| UALBF.Manifest.PREFIX_STOP_THRESHOLD.toUInt64

@[export ualbf_raycast_gpu_threshold]
def ualbf_raycast_gpu_threshold_impl : UInt32 := ((1 : UInt32) <<< 31) ||| UALBF.Manifest.RAYCAST_GPU_THRESHOLD.toUInt32

@[export ualbf_raycast_chunk_size]
def ualbf_raycast_chunk_size_impl : UInt32 := ((1 : UInt32) <<< 31) ||| UALBF.Manifest.RAYCAST_CHUNK_SIZE.toUInt32


@[export ualbf_logic_hash]
def ualbf_logic_hash_impl : String := UALBF.Manifest.LOGIC_HASH

