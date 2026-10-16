import Mathlib.Data.Nat.Basic
import Mathlib.Data.Nat.Factorization.Basic
import UALBF.Basic
import UALBF.Engine.SearchState
import UALBF.Engine.CyclotomicGraph
import UALBF.QPN.PrasadSunitha

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
  Rule A soundness for a bounded search.
  If the partial product `s.n_l` already exceeds `target_bound`, every positive
  extension `N` of it (every `N` with `s.n_l ∣ N`) also exceeds `target_bound`, so
  pruning the branch loses no candidate `N ≤ target_bound`. This says nothing about
  quasiperfect numbers above `target_bound`.
-/
theorem ruleA_safe (s : SearchState) (target_bound : Nat) (N : Nat)
    (h_prune : s.n_l > target_bound)
    (h_ext : s.n_l ∣ N)
    (h_pos : 0 < N) :
    target_bound < N :=
  Nat.lt_of_lt_of_le h_prune (Nat.le_of_dvd h_pos h_ext)

/--
  Cyclotomic forcing lemma used by Rule B.
  This is `CyclotomicGraph.forced_inclusion`: an even exponent `2e` on `p` forces a
  prime `q ≡ 1 (mod d)` to divide `σ(N)`. It is a statement about `σ(N)`, not about
  the factors of `N`, so it does not by itself justify `ruleB_pruning`; that pruning
  rule has no soundness proof.
-/
theorem ruleB_safe {p e N : ℕ}
    (hp_prime : p.Prime)
    (hp_ge_3 : 3 ≤ p)
    (he1 : 1 ≤ e)
    (h_exact : ExactValuation p (2 * e) N)
    (h_qpn : IsQuasiperfect N) (d : ℕ) (hd : d ∣ (2 * e + 1)) (hd1 : 1 < d) :
    ∃ q, q.Prime ∧ q % d = 1 ∧ q ∣ sigma N :=
  CyclotomicGraph.forced_inclusion hp_prime hp_ge_3 he1 h_exact h_qpn d hd hd1

/--
  Prasad-Sunitha Search Tree Component Bounding Theorem.
  Proves that candidate integers constructed from a finite search component list
  cannot exceed the maximum factor count derived from the component list length,
  and therefore cannot yield a quasiperfect number when the component count is strictly
  less than the required Prasad-Sunitha bound.
-/
theorem prasad_sunitha_component_bound {N : ℕ} {components : Finset ℕ} {bound : ℕ}
    (h_sub : N.primeFactors ⊆ components)
    (h_ps_bound : bound ≤ N.primeFactors.card) :
    bound ≤ components.card :=
  le_trans h_ps_bound (Finset.card_le_card h_sub)

/--
  Corollary connecting a quasiperfect candidate integer N coprime to 15
  and constructed from a component list to the Prasad-Sunitha bound (15).
-/
theorem prasad_sunitha_component_bound_qpn {N : ℕ} {components : Finset ℕ}
    (h_qpn : IsQuasiperfect N)
    (h_coprime : N.gcd 15 = 1)
    (h_sub : N.primeFactors ⊆ components) :
    UALBF.Manifest.PRASAD_SUNITHA_PROOF_BOUND ≤ components.card :=
  prasad_sunitha_component_bound h_sub (UALBF.QPN.PrasadSunitha.qpn_coprime_15_omega_bound h_qpn h_coprime)

def dfs_step : SearchM Unit := do
  let s ← get
  set s

@[export ualbf_search_monad_step]
def ualbf_search_monad_step_impl (_ctx : UInt64) : IO Unit :=
  return ()

end UALBF.Engine
