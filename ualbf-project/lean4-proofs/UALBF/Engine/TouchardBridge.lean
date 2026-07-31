import UALBF.Basic
import Mathlib.Data.ZMod.Basic
import Mathlib.Data.Nat.Factorization.Basic
import UALBF.FFI
import UALBF.QPN.TouchardQPN
import Mathlib.Tactic.Ring

namespace UALBF.Engine.TouchardBridge

open UALBF
open UALBF.QPN.TouchardQPN

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
