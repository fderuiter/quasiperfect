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

set_option linter.unusedVariables false

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
  intro h_exact
  have h_dvd := SieveSoundness.exact_val_sigma_dvd hp_prime h_exact
  obtain ⟨q, hq_prime, hq_mod_d, hq_dvd_sigma_p⟩ := S.forced_implies_dvd p e d h_forced
  have h_q_dvd_sigma_N : q ∣ sigma N := dvd_trans hq_dvd_sigma_p h_dvd
  have h_forced_prime := forced_inclusion hp_prime hp_ge_3 he1 h_exact h_qpn d hd hd1
  obtain ⟨q_fi, hq_fi_prime, hq_fi_mod_d, h_q_fi_dvd_sigma_N⟩ := h_forced_prime
  exact S.obstruction N d q h_qpn h_cond hq_prime hq_mod_d h_q_dvd_sigma_N

end UALBF.Engine.CyclotomicGraph
