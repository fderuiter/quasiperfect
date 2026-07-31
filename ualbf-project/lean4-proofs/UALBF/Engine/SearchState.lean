-- AUTO-GENERATED from schema_manifest.json. DO NOT EDIT.
set_option linter.all false

import Mathlib.Data.Nat.Basic
import UALBF.FFI

namespace UALBF.Engine

structure SearchState where
  n_l : Nat
  s_l : Nat
  last_idx : Nat
  factors : Array UInt64
  sigma_factors : Array Nat
  sigma_factors_u64 : Array UInt64
  active_mask : Array UInt64
  sigma_mod24 : UInt64
deriving Inhabited, Repr

structure SearchStateTransport where
  n_l : UALBF.FFI.U512
  s_l : UALBF.FFI.U512
  last_idx : Nat
  factors : Array UInt64
  sigma_factors : Array UALBF.FFI.U512
  sigma_factors_u64 : Array UInt64
  active_mask : Array UInt64
  sigma_mod24 : UInt64
deriving Inhabited

def SearchStateTransport.toNative (t : SearchStateTransport) : SearchState := {
  n_l := UALBF.FFI.fromU512 t.n_l,
  s_l := UALBF.FFI.fromU512 t.s_l,
  last_idx := t.last_idx,
  factors := t.factors,
  sigma_factors := t.sigma_factors.map UALBF.FFI.fromU512,
  sigma_factors_u64 := t.sigma_factors_u64,
  active_mask := t.active_mask,
  sigma_mod24 := t.sigma_mod24,
}

end UALBF.Engine
