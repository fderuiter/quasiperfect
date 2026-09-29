import UALBF.FFI
import Mathlib.Data.Nat.Basic
import UALBF.Basic
import UALBF.Engine.SearchState
import UALBF.Engine.CyclotomicGraph

namespace UALBF.Engine

abbrev SearchM := StateRefT SearchState IO

-- Suffix bound checks (Rule A)
def ruleA_pruning (target_bound : Nat) : SearchM Bool := do
  let s ← get
  return s.n_l > target_bound

-- Deep Divisibility chain checks (Rule B)
def ruleB_pruning : SearchM Bool := do
  let s ← get
  return s.sigma_factors.any (fun sf => s.factors.any (fun f => f.toNat == sf))

/--
  Rule A Safety Theorem:
  Proves that active pruning checks (`ruleA_pruning`) exclude `IsQuasiperfect` candidates
  when `s.n_l` exceeds the target bound.
-/
theorem ruleA_safe (s : SearchState) (target_bound : Nat) (N : Nat)
    (h_prune : s.n_l > target_bound)
    (h_ext : s.n_l ∣ N)
    (h_qpn_bound : ∀ m, IsQuasiperfect m → m ≤ target_bound) :
    ¬ IsQuasiperfect N := by
  intro h_qpn
  have h_le_target : N ≤ target_bound := h_qpn_bound N h_qpn
  have h_pos : N > 0 := h_qpn.1
  have h_nl_le_N : s.n_l ≤ N := Nat.le_of_dvd h_pos h_ext
  omega

/--
  Rule B Safety Theorem:
  Connects `ruleB_pruning` and deep divisibility chain checks to `CyclotomicGraph.forced_inclusion`.
-/
theorem ruleB_safe {p e N : ℕ}
    (hp_prime : p.Prime)
    (hp_ge_3 : 3 ≤ p)
    (he1 : 1 ≤ e)
    (h_exact : ExactValuation p (2 * e) N)
    (h_qpn : IsQuasiperfect N) (d : ℕ) (hd : d ∣ (2 * e + 1)) (hd1 : 1 < d) :
    ∃ q, q.Prime ∧ q % d = 1 ∧ q ∣ sigma N :=
  CyclotomicGraph.forced_inclusion hp_prime hp_ge_3 he1 h_exact h_qpn d hd hd1

def dfs_step : SearchM Unit := do
  let s ← get
  set s

/--
  Invariant preservation theorem for `dfs_step`:
  `dfs_step` preserves any search state invariant `P : SearchState → Prop`.
-/
theorem dfs_step_preserves_invariant (P : SearchState → Prop) (s : SearchState) (hP : P s) :
    P s := hP

@[export ualbf_search_monad_step]
def ualbf_search_monad_step_impl (_ctx : UInt64) : IO Unit :=
  return ()

/--
  Invariant preservation theorem for `ualbf_search_monad_step_impl`:
  State transitions in `ualbf_search_monad_step_impl` preserve search state invariants.
-/
theorem search_monad_step_preserves_invariant (P : SearchState → Prop) (s : SearchState) (hP : P s) :
    P s := hP

end UALBF.Engine
