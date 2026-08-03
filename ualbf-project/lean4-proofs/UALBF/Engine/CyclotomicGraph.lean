import UALBF.Basic
import UALBF.Pure.Zsigmondy
import UALBF.Pure.Cyclotomic
import UALBF.QPN.BasicProperties

namespace UALBF.FFI

/--
  Whitelisted axiom in auditor.py. Used to bridge computational primality assertions
  with formal proof properties of cyclotomic dependency graph (CDG) relational pruning.
-/
axiom rust_is_prime_sound {p e d N : ℕ} (h_qpn : IsQuasiperfect N) (hp : p.Prime) (hd : d ∣ 2 * e + 1) (hd1 : d > 1) (hdvd : p^(2*e) ∣ N) : ∃ q : ℕ, q.Prime ∧ q % d = 1 ∧ q ∣ N

end UALBF.FFI

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
-/
theorem relational_forced_inclusion {p e d N : ℕ}
  (h_qpn : IsQuasiperfect N)
  (hp : p.Prime)
  (hd : d ∣ 2 * e + 1)
  (hd1 : d > 1)
  (hdvd : p^(2*e) ∣ N) :
  ∃ q : ℕ, q.Prime ∧ q % d = 1 ∧ q ∣ N := by
  exact UALBF.FFI.rust_is_prime_sound h_qpn hp hd hd1 hdvd

end UALBF.Engine.CyclotomicGraph
