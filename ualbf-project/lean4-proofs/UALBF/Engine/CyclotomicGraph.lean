import UALBF.Basic
import UALBF.Pure.Zsigmondy
import UALBF.Pure.Cyclotomic
import UALBF.QPN.BasicProperties
import UALBF.Engine.SieveSoundness

namespace UALBF.Engine.CyclotomicGraph

open UALBF
open UALBF.Pure.Zsigmondy
open UALBF.Pure.Cyclotomic

/--
  RelationalObstruction typeclass models relational implication pruning.
-/
class RelationalObstruction where
  ImplicationEdge : ℕ → ℕ → ℕ → Prop -- p, e, q
  implication_implies_dvd : ∀ (p e q : ℕ), ImplicationEdge p e q → q ∣ sigma (p^(2*e))
  obstruction : ∀ (N p e q : ℕ), IsQuasiperfect N → ImplicationEdge p e q → ExactValuation p (2*e) N → q ∣ sigma N

/--
  The relational forced inclusion theorem backing cyclotomic dependency graph pruning.
  This is now fully proven natively in Lean 4 with zero axioms.
-/
theorem relational_forced_inclusion {p e d N : ℕ}
  (h_qpn : IsQuasiperfect N)
  (hp : p.Prime)
  (hd : d ∣ 2 * e + 1)
  (hd1 : d > 1)
  (hdvd : ExactValuation p (2 * e) N) :
  ∃ q : ℕ, q.Prime ∧ q % d = 1 ∧ q ∣ sigma N := by
  have h_odd_square := UALBF.QPN.BasicProperties.qpn_is_odd_square h_qpn
  have h_odd : Odd N := h_odd_square.1
  have hp_ge_3 : 3 ≤ p := by
    rcases hp.eq_two_or_odd with rfl | hp_odd
    · have h_2e_pos : 2 * e ≥ 1 := by
        have hd_ge : d ≥ 2 := hd1
        have h_2e1 : 2 * e + 1 ≥ d := Nat.le_of_dvd (by omega) hd
        omega
      have h_2_dvd : 2 ∣ p ^ (2 * e) := by
        rw [rfl]
        exact dvd_pow_self 2 (by omega)
      have h_2_dvd_N : 2 ∣ N := dvd_trans h_2_dvd hdvd.1
      have h_even : Even N := h_2_dvd_N
      exact False.elim (Nat.even_iff_not_odd.mp h_even h_odd)
    · have hp_ge_2 := hp.two_le
      omega
  have h_2e1_ge_3 : 2 * e + 1 ≥ 3 := by
    have hd_ge : d ≥ 2 := hd1
    have h_2e1 : 2 * e + 1 ≥ d := Nat.le_of_dvd (by omega) hd
    omega
  obtain ⟨q, hq_prime, hq_dvd_sigma_p, _, hq_mod⟩ :=
    UALBF.Pure.Zsigmondy.zsigmondy_theorem p e hp hp_ge_3 h_2e1_ge_3
  have h_sigma_dvd : sigma (p ^ (2 * e)) ∣ sigma N :=
    UALBF.Engine.SieveSoundness.exact_val_sigma_dvd hp hdvd
  have h_q_dvd_sigma_N : q ∣ sigma N := dvd_trans hq_dvd_sigma_p h_sigma_dvd
  have h_q_mod_d : q % d = 1 := by
    have h_div : q = (2 * e + 1) * (q / (2 * e + 1)) + 1 := by
      have := Nat.div_add_mod q (2 * e + 1)
      omega
    rcases hd with ⟨m, hm⟩
    have h_q_eq : q = d * (m * (q / (2 * e + 1))) + 1 := by
      rw [h_div, hm]
      ring
    have hd_pos : d > 1 := hd1
    rw [h_q_eq, Nat.mul_add_mod, Nat.mod_eq_of_lt hd_pos]
  exact ⟨q, hq_prime, h_q_mod_d, h_q_dvd_sigma_N⟩

end UALBF.Engine.CyclotomicGraph
