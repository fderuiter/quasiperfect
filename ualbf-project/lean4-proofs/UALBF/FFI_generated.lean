-- AUTO-GENERATED from schema_manifest.json. DO NOT EDIT.
set_option linter.all false

namespace UALBF.FFI

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

macro "u512_omega_prep" : tactic => `(tactic|
  have h2_64 : 2^64 = 18446744073709551616 := rfl;
  have h2_128 : 2^128 = 340282366920938463463374607431768211456 := rfl;
  have h2_192 : 2^192 = 6277101735386680763835789423207666416102355444464034512896 := rfl;
  have h2_256 : 2^256 = 115792089237316195423570985008687907853269984665640564039457584007913129639936 := rfl;
  have h2_320 : 2^320 = 2135987035920910082395021706169552114602704522356652769947041607822219725780640550022962086936576 := rfl;
  have h2_384 : 2^384 = 39402006196394479212279040100143613805079739270465446667948293404245721771497210611414266254884915640806627990306816 := rfl;
  have h2_448 : 2^448 = 726838724295606890549323807888004534353641360687318060281490199180639288113397923326191050713763565560762521606266177933534601628614656 := rfl;
  rw [h2_64, h2_128, h2_192, h2_256, h2_320, h2_384, h2_448] at *
)

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
  u512_omega_prep
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
  u512_omega_prep
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
  u512_omega_prep
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
  u512_omega_prep
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
  u512_omega_prep
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
  u512_omega_prep
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
  u512_omega_prep
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
  u512_omega_prep
  omega

@[extern "rust_u512_to_hex"]
opaque U512.toHex (u : @& U512) : String

def parseHexChar (c : Char) : Nat :=
  if '0' <= c && c <= '9' then c.toNat - '0'.toNat
  else if 'a' <= c && c <= 'f' then 10 + (c.toNat - 'a'.toNat)
  else if 'A' <= c && c <= 'F' then 10 + (c.toNat - 'A'.toNat)
  else 0

def parseHex (s : String) : Nat :=
  let s := if s.startsWith "0x" || s.startsWith "0X" then s.drop 2 else s
  s.foldl (fun acc c => acc * 16 + parseHexChar c) 0

def fromHexU512 (u : U512) : Nat :=
  parseHex (u.toHex)

@[implemented_by fromHexU512]
def fromU512 (u : U512) : Nat :=
  u.w0.toNat +
  u.w1.toNat * (2 ^ 64) +
  u.w2.toNat * (2 ^ 128) +
  u.w3.toNat * (2 ^ 192) +
  u.w4.toNat * (2 ^ 256) +
  u.w5.toNat * (2 ^ 320) +
  u.w6.toNat * (2 ^ 384) +
  u.w7.toNat * (2 ^ 448)

def toU512 (n : Nat) : U512 :=
  U512.mk
    (n % 2^64).toUInt64
    ((n / 2^64) % 2^64).toUInt64
    ((n / 2^128) % 2^64).toUInt64
    ((n / 2^192) % 2^64).toUInt64
    ((n / 2^256) % 2^64).toUInt64
    ((n / 2^320) % 2^64).toUInt64
    ((n / 2^384) % 2^64).toUInt64
    ((n / 2^448) % 2^64).toUInt64

def SCHEMA_MANIFEST_HASH : String := "c57a8afb8ff0f71a7a20b7ad46608995ce40c66e7ca69dd7c7ed2f33e78c92af"

end UALBF.FFI
