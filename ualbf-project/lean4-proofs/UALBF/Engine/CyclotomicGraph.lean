import Mathlib.Data.Nat.Basic
import UALBF.Basic
import UALBF.Pure.Zsigmondy
import UALBF.Pure.Cyclotomic
import UALBF.Engine.SieveSoundness

namespace UALBF.Engine.CyclotomicGraph

open UALBF
open UALBF.Pure.Zsigmondy
open UALBF.Pure.Cyclotomic

/--
  Local helper lemma that transports a modular condition from n = 2*e+1
  down to a divisor d: given q % n = 1, d ∣ n, 1 < d, and 1 ≤ q,
  conclude q % d = 1.
-/
lemma mod_divisor_transport {q n d : ℕ}
  (hqn : q % n = 1)
  (hdn : d ∣ n)
  (hd1 : 1 < d)
  (hq1 : 1 ≤ q) :
  q % d = 1 := by
  have hn_dvd : n ∣ q - 1 := by
    have hq_eq := Nat.div_add_mod q n
    rw [hqn] at hq_eq
    have : q - 1 = n * (q / n) := by omega
    exact ⟨q / n, this⟩
  have hd_dvd : d ∣ q - 1 := dvd_trans hdn hn_dvd
  obtain ⟨k, hk⟩ := hd_dvd
  have h_q_eq : q = d * k + 1 := by omega
  rw [h_q_eq, Nat.mul_add_mod, Nat.mod_eq_of_lt (by omega)]


/--
  The relational forced inclusion theorem backing cyclotomic dependency graph pruning.
  Proves that any divisor d of the cyclotomic index (2e+1) must yield a prime q % d = 1.
-/
theorem forced_inclusion {p e N : ℕ}
  (hp_prime : p.Prime)
  (hp_ge_3 : 3 ≤ p)
  (he1 : 1 ≤ e)
  (h_exact : ExactValuation p (2 * e) N)
  (h_qpn : IsQuasiperfect N) :
  ∀ d, d ∣ (2 * e + 1) → 1 < d → ∃ q, q.Prime ∧ q % d = 1 ∧ q ∣ sigma N := by
  have _ := h_qpn
  have h_2e1_ge_3 : 2 * e + 1 ≥ 3 := by omega
  obtain ⟨q, hq_prime, hq_dvd_sigma_p, _, hq_mod_2e1⟩ :=
    zsigmondy_theorem p e hp_prime hp_ge_3 h_2e1_ge_3
  have h_sigma_dvd : sigma (p ^ (2 * e)) ∣ sigma N :=
    SieveSoundness.exact_val_sigma_dvd hp_prime h_exact
  have h_q_dvd_sigma_N : q ∣ sigma N := dvd_trans hq_dvd_sigma_p h_sigma_dvd
  intro d hd hd1
  have h_q1 : 1 ≤ q := hq_prime.one_lt.le
  have hq_mod_d : q % d = 1 := mod_divisor_transport hq_mod_2e1 hd hd1 h_q1
  exact ⟨q, hq_prime, hq_mod_d, h_q_dvd_sigma_N⟩


/--
  Single-step force relation in the cyclotomic dependency graph:
  Component (p1, e1) forces prime p2 if p2 is prime and divides sigma (p1^(2*e1)).
-/
def SingleStepForce (p1 e1 p2 : ℕ) : Prop :=
  p2.Prime ∧ p2 ∣ sigma (p1 ^ (2 * e1))

/--
  Multi-step transitive reachability path relation in the component graph with valuation propagation.
  `TransitiveReach N p1 e1 p2 e2` holds if component (p1, e1) transitively reaches (p2, e2)
  in N, propagating prime divisibility and exact component valuations along the path.
-/
inductive TransitiveReach (N : ℕ) : ℕ → ℕ → ℕ → ℕ → Prop
  | step (p1 e1 p2 e2 : ℕ)
      (hp1 : p1.Prime)
      (h_exact1 : ExactValuation p1 (2 * e1) N)
      (h_step : SingleStepForce p1 e1 p2)
      (hp2 : p2.Prime)
      (h_exact2 : ExactValuation p2 (2 * e2) N) : TransitiveReach N p1 e1 p2 e2
  | trans (p1 e1 p2 e2 p3 e3 : ℕ)
      (h1 : TransitiveReach N p1 e1 p2 e2)
      (h2 : TransitiveReach N p2 e2 p3 e3) : TransitiveReach N p1 e1 p3 e3

/--
  Single-step forced inclusion theorem:
  If component (p1, e1) is exact-valued in N and forces p2, then p2 divides sigma N.
-/
theorem single_step_forced_inclusion {p1 e1 p2 N : ℕ}
  (hp1 : p1.Prime)
  (h_exact : ExactValuation p1 (2 * e1) N)
  (h_step : SingleStepForce p1 e1 p2) :
  p2 ∣ sigma N := by
  have h_dvd := SieveSoundness.exact_val_sigma_dvd hp1 h_exact
  exact dvd_trans h_step.2 h_dvd

/--
  Transitive forced inclusion helper lemma:
  Single-step forced inclusion preserves component divisibility in N.
-/
theorem transitive_forced_inclusion {p1 e1 p2 N : ℕ}
  (hp1 : p1.Prime)
  (h_exact1 : ExactValuation p1 (2 * e1) N)
  (h_step : SingleStepForce p1 e1 p2) :
  p2 ∣ sigma N :=
  single_step_forced_inclusion hp1 h_exact1 h_step

/--
  Helper lemma: Any component (p2, e2) transitively reached in N is prime and exact-valued in N.
-/
lemma transitive_reach_target_exact {p1 e1 p2 e2 N : ℕ}
  (h_reach : TransitiveReach N p1 e1 p2 e2) :
  p2.Prime ∧ ExactValuation p2 (2 * e2) N := match h_reach with
  | TransitiveReach.step _ _ _ _ _ _ _ hp2 h_exact2 => ⟨hp2, h_exact2⟩
  | TransitiveReach.trans _ _ _ _ _ _ _ h2 => transitive_reach_target_exact h2

/--
  Helper lemma: Any component (p1, e1) starting a reachability path in N is prime and exact-valued in N.
-/
lemma transitive_reach_start_exact {p1 e1 p2 e2 N : ℕ}
  (h_reach : TransitiveReach N p1 e1 p2 e2) :
  p1.Prime ∧ ExactValuation p1 (2 * e1) N := match h_reach with
  | TransitiveReach.step _ _ _ _ hp1 h_exact1 _ _ _ => ⟨hp1, h_exact1⟩
  | TransitiveReach.trans _ _ _ _ _ _ h1 _ => transitive_reach_start_exact h1

/--
  Transitive reachability soundness theorem:
  Multi-step transitive reachability paths in the component graph preserve forced inclusions.
  If component (p1, e1) reaches (p2, e2) transitively in N and (p2, e2) forces prime q,
  then q divides sigma N.
-/
theorem transitive_reachability_soundness {p1 e1 p2 e2 q N : ℕ}
  (hp1 : p1.Prime)
  (h_exact1 : ExactValuation p1 (2 * e1) N)
  (h_reach : TransitiveReach N p1 e1 p2 e2)
  (h_force : SingleStepForce p2 e2 q) :
  q ∣ sigma N := by
  have _ := hp1
  have _ := h_exact1
  obtain ⟨hp2, h_exact2⟩ := transitive_reach_target_exact h_reach
  exact single_step_forced_inclusion hp2 h_exact2 h_force


/--
  RelationalObstruction typeclass models relational implication pruning.
  Mirrors the four-field shape of ModularSieve.
-/
class RelationalObstruction where
  cond : ℕ → Prop
  ForcedComponent : ℕ → ℕ → ℕ → Prop
  forced_implies_dvd : ∀ (p e d : ℕ), ForcedComponent p e d → ∃ q, q.Prime ∧ q % d = 1 ∧ q ∣ sigma (p^(2*e))
  obstruction : ∀ (N d q : ℕ), IsQuasiperfect N → cond N → q.Prime → q % d = 1 → ¬ (q ∣ sigma N)

/--
  Generic soundness theorem for relational sieve obstruction.
  Concludes that a component satisfying RelationalObstruction cannot be exactly valued.
-/
theorem relational_sieve_soundness_generic [S : RelationalObstruction] {N p e d : ℕ}
  (h_qpn : IsQuasiperfect N)
  (h_cond : S.cond N)
  (hp_prime : p.Prime)
  (hp_ge_3 : 3 ≤ p)
  (he1 : 1 ≤ e)
  (hd : d ∣ 2 * e + 1)
  (hd1 : 1 < d)
  (h_forced : S.ForcedComponent p e d) :
  ¬ ExactValuation p (2 * e) N := by
  have _ := hp_ge_3
  have _ := he1
  have _ := hd
  have _ := hd1
  intro h_exact
  have h_dvd := SieveSoundness.exact_val_sigma_dvd hp_prime h_exact
  obtain ⟨q, hq_prime, hq_mod_d, hq_dvd_sigma_p⟩ := S.forced_implies_dvd p e d h_forced
  have h_q_dvd_sigma_N : q ∣ sigma N := dvd_trans hq_dvd_sigma_p h_dvd
  exact S.obstruction N d q h_qpn h_cond hq_prime hq_mod_d h_q_dvd_sigma_N

theorem is_cdg_forced_pruned {p1 e1 p2 N : ℕ}
  (hp1 : p1.Prime)
  (h_exact1 : ExactValuation p1 (2 * e1) N)
  (h_step : SingleStepForce p1 e1 p2) :
  p2 ∣ sigma N :=
  single_step_forced_inclusion hp1 h_exact1 h_step

theorem zsigmondy_preconditions_satisfied (p d : ℕ)
  (hp : 3 ≤ p) (hp_odd : Odd p) (hd : 3 ≤ d) :
  3 ≤ p ∧ Odd p ∧ 3 ≤ d :=
  ⟨hp, hp_odd, hd⟩

theorem proof_verify_zsigmondy_preconditions (p d : ℕ)
  (hp : 3 ≤ p) (hp_odd : Odd p) (hd : 3 ≤ d) :
  3 ≤ p ∧ Odd p ∧ 3 ≤ d :=
  zsigmondy_preconditions_satisfied p d hp hp_odd hd

end UALBF.Engine.CyclotomicGraph
