import Mathlib.Data.Nat.Basic
import Mathlib.NumberTheory.Divisors
import Mathlib.NumberTheory.ArithmeticFunction.Misc
import Mathlib.Algebra.BigOperators.Ring.Nat
import Mathlib.Data.Nat.Factorization.Basic
import Mathlib.Tactic.Ring
import Mathlib.Tactic.Linarith
import Mathlib.Data.ZMod.Basic
import Mathlib.NumberTheory.LegendreSymbol.Basic
import UALBF.Basic

/-!
# Pure Arithmetic Helpers

Generic parity, modular arithmetic, and factorization lemmas.
No QPN hypothesis — pure, upstreamable mathematics.
-/

namespace UALBF.Pure.Arithmetic

open Finset
open Nat
open UALBF

/-- Helper: sigma is odd iff the number of its odd divisors is odd. -/
lemma odd_sigma_iff_odd_card_odd_divisors (n : ℕ) :
  Odd (sigma n) ↔ Odd (n.divisors.filter Odd).card := by
  unfold sigma
  rw [odd_sum_iff_odd_card_odd]

/-- Helper: σ(n) is odd iff its factorization implies all odd primes have even multiplicity -/
lemma odd_prod_iff {α : Type*} [DecidableEq α] {s : Finset α} {f : α → ℕ} :
  Odd (∏ x ∈ s, f x) ↔ ∀ x ∈ s, Odd (f x) := by
  induction s using Finset.induction_on with
  | empty => simp
  | insert a s' ha ih =>
    rw [prod_insert ha, Nat.odd_mul, ih]
    constructor
    · rintro ⟨h1, h2⟩ x hx
      rw [mem_insert] at hx
      rcases hx with rfl | hx
      · exact h1
      · exact h2 x hx
    · intro h
      exact ⟨h a (mem_insert_self a s'), fun x hx => h x (mem_insert_of_mem hx)⟩

lemma odd_sum_pow_of_odd {p v : ℕ} (h_odd : Odd p) :
  Odd (∑ k ∈ .range (v + 1), p ^ k) ↔ Even v := by
  induction v with
  | zero => simp
  | succ v ih =>
    rw [sum_range_succ, Nat.odd_add]
    have hpk : Odd (p ^ (v + 1)) := Odd.pow h_odd
    have h_not_even : ¬ Even (p ^ (v + 1)) := fun h => by
      rcases h with ⟨k, hk⟩
      rcases hpk with ⟨m, hm⟩
      omega
    have h1 : (Even (p ^ (v + 1)) ↔ False) := iff_false_intro h_not_even
    rw [h1, iff_false]
    have h2 : Even v ↔ ¬ Even (v + 1) := by
      constructor
      · intro hv hv1; rcases hv with ⟨a, rfl⟩; rcases hv1 with ⟨b, hb⟩; omega
      · intro _hnv1; cases Nat.even_or_odd v with
        | inl he => exact he
        | inr ho => rcases ho with ⟨a, rfl⟩; have : Even (2 * a + 1 + 1) := ⟨a + 1, by omega⟩; contradiction
    rw [ih, h2]
    exact not_not

lemma even_sum_pow_of_even {p v : ℕ} (h_even : Even p) :
  Odd (∑ k ∈ .range (v + 1), p ^ k) := by
  induction v with
  | zero => simp
  | succ v ih =>
    rw [sum_range_succ, Nat.odd_add]
    have hpk : Even (p ^ (v + 1)) := Even.pow_of_ne_zero h_even (Nat.succ_ne_zero v)
    simp [ih, hpk]

lemma prime_even_eq_two {p : ℕ} (hp : p.Prime) (he : Even p) : p = 2 := by
  have h_dvd : 2 ∣ p := even_iff_two_dvd.mp he
  rcases h_dvd with ⟨a, ha⟩
  have h_or := hp.eq_two_or_odd
  cases h_or with
  | inl h2 => exact h2
  | inr ho => omega

lemma odd_sigma_iff_factorization (n : ℕ) (hn : n ≠ 0) :
  Odd (sigma n) ↔ ∀ p ∈ n.primeFactors, p ≠ 2 → Even (n.factorization p) := by
  unfold sigma
  rw [Nat.sum_divisors hn]
  rw [odd_prod_iff]
  constructor
  · intro h p hp hp2
    have hp_prime : p.Prime := Nat.prime_of_mem_primeFactors hp
    have h_odd_sum := h p hp
    have hp_odd : Odd p := by
      cases Nat.even_or_odd p with
      | inl he =>
        have _h_eq_2 := prime_even_eq_two hp_prime he
        contradiction
      | inr ho => exact ho
    have h_eval := (@odd_sum_pow_of_odd p (n.factorization p) hp_odd).mp h_odd_sum
    exact h_eval
  · intro h p hp
    have hp_prime : p.Prime := Nat.prime_of_mem_primeFactors hp
    cases Nat.even_or_odd p with
    | inl he =>
      have h_even_sum := @even_sum_pow_of_even p (n.factorization p) he
      exact h_even_sum
    | inr ho =>
      have hp2 : p ≠ 2 := by
        intro heq
        rw [heq] at ho
        contradiction
      have heven_fac := h p hp hp2
      have h_odd_sum := (@odd_sum_pow_of_odd p (n.factorization p) ho).mpr heven_fac
      exact h_odd_sum

lemma extract_odd_factor (m : ℕ) (h_pos : m > 0) : ∃ e u : ℕ, m = 2 ^ e * u ∧ ¬ 2 ∣ u := by
  induction m using Nat.strong_induction_on with
  | h m ih =>
    cases Nat.even_or_odd m with
    | inr h_odd =>
      use 0, m
      have h_not_dvd : ¬ 2 ∣ m := by
        intro h_dvd
        rcases h_dvd with ⟨c, hc⟩
        rcases h_odd with ⟨d, hd⟩
        omega
      exact ⟨by ring, h_not_dvd⟩
    | inl h_even =>
      have h_m_eq : m = 2 * (m / 2) := by
        rcases h_even with ⟨c, hc⟩
        omega
      have hm2 : m / 2 < m := Nat.div_lt_self h_pos (by decide)
      have hm2_pos : m / 2 > 0 := by
        rcases h_even with ⟨c, hc⟩
        omega
      rcases ih (m / 2) hm2 hm2_pos with ⟨e, u, h_eq, h_u⟩
      use e + 1, u
      constructor
      · calc m = 2 * (m / 2) := h_m_eq
             _ = 2 * (2 ^ e * u) := by rw [h_eq]
             _ = 2 ^ (e + 1) * u := by ring
      · exact h_u

lemma odd_even_factorization_is_square (m : ℕ) (h_pos : m > 0)
  (h_even : ∀ p ∈ m.primeFactors, Even (m.factorization p)) :
  ∃ k, m = k ^ 2 := by
  have h_all : ∀ p, Even (m.factorization p) := by
    intro p
    by_cases hp : p ∈ m.primeFactors
    · exact h_even p hp
    · have hp_not_supp : p ∉ m.factorization.support := hp
      have h_zero : m.factorization p = 0 := Finsupp.notMem_support_iff.mp hp_not_supp
      rw [h_zero]
      exact Even.zero
  obtain ⟨b, a, hb_pos, ha_pos, hab, hb_sqfree⟩ := Nat.sq_mul_squarefree_of_pos h_pos
  have hb_one : b = 1 := by
    by_cases hb_one' : b = 1
    · exact hb_one'
    · have h_exists : ∃ p, p.Prime ∧ p ∣ b := Nat.exists_prime_and_dvd hb_one'
      rcases h_exists with ⟨p, hp_prime, hp_dvd⟩
      have h_m_fac : Even (m.factorization p) := h_all p
      have hab_symm : m = a ^ 2 * b := hab.symm
      have _h_m_fac_eq : m.factorization p = 2 * a.factorization p + b.factorization p := by
        rw [hab_symm]
        have ha2_pos_ne_zero : a ^ 2 ≠ 0 := by
          intro h
          have _h_a_zero : a = 0 := by
            rcases a with _ | _
            · rfl
            · contradiction
          omega
        have hb_pos_ne_zero : b ≠ 0 := by omega
        rw [Nat.factorization_mul ha2_pos_ne_zero hb_pos_ne_zero]
        simp only [Finsupp.coe_add, Pi.add_apply, Nat.factorization_pow, Finsupp.coe_smul, Pi.smul_apply, smul_eq_mul]
      have h_2a_even : Even (2 * a.factorization p) := even_two_mul _
      have h_b_even : Even (b.factorization p) := by
        rcases h_m_fac with ⟨k_m, hk_m⟩
        rcases h_2a_even with ⟨k_a, hk_a⟩
        have hk_b : b.factorization p = (k_m - k_a) + (k_m - k_a) := by omega
        exact ⟨k_m - k_a, hk_b⟩
      have _hb_le_one : b.factorization p ≤ 1 := hb_sqfree.natFactorization_le_one p
      have _hb_zero : b.factorization p = 0 := by
        rcases h_b_even with ⟨k_b, hk_b⟩
        omega
      have _hb_fac_ge_one : b.factorization p ≥ 1 := by
        rwa [Nat.Prime.dvd_iff_one_le_factorization hp_prime hb_pos.ne.symm] at hp_dvd
      omega
  use a
  calc m = a ^ 2 * b := hab.symm
       _ = a ^ 2 * 1 := by rw [hb_one]
       _ = a ^ 2 := by ring

/-- Helper: factorization of odd primes being even is equivalent to n being a square or double square -/
lemma factorization_even_iff_square_or_double_square (n : ℕ) (hn : n ≠ 0) :
  (∀ p ∈ n.primeFactors, p ≠ 2 → Even (n.factorization p)) ↔
  (∃ m : ℕ, n = m ^ 2) ∨ (∃ m : ℕ, n = 2 * m ^ 2) := by
  have hn_pos : n > 0 := Nat.pos_iff_ne_zero.mpr hn
  constructor
  · intro h
    rcases extract_odd_factor n hn_pos with ⟨e, u, hn_eq, h_not_dvd⟩
    have hu_pos : u > 0 := by
      have _h2e : 2 ^ e > 0 := Nat.pos_of_ne_zero (pow_ne_zero _ (by decide))
      exact Nat.pos_of_ne_zero (fun hu_zero => by rw [hu_zero, mul_zero] at hn_eq; omega)
    have h_u_even : ∀ p ∈ u.primeFactors, Even (u.factorization p) := by
      intro p hp
      have hp_prime : p.Prime := Nat.prime_of_mem_primeFactors hp
      have h2 : p ≠ 2 := by
        intro hp2
        subst hp2
        have hdvd_u : 2 ∣ u := Nat.dvd_of_mem_primeFactors hp
        exact h_not_dvd hdvd_u
      have hdvd_n : p ∣ n := by rw [hn_eq]; exact dvd_mul_of_dvd_right (Nat.dvd_of_mem_primeFactors hp) _
      have hp_n_mem : p ∈ n.primeFactors := Nat.mem_primeFactors.mpr ⟨hp_prime, hdvd_n, hn⟩
      have _hn_even := h p hp_n_mem h2
      have hn_eq_fac : n.factorization p = u.factorization p := by
        have h2_ne_zero : 2 ≠ 0 := by decide
        have h_pow_ne_zero : 2 ^ e ≠ 0 := pow_ne_zero e h2_ne_zero
        have hu_ne_zero : u ≠ 0 := by omega
        rw [hn_eq]
        rw [Nat.factorization_mul h_pow_ne_zero hu_ne_zero]
        simp only [Finsupp.coe_add, Pi.add_apply, Nat.factorization_pow, Finsupp.coe_smul, Pi.smul_apply, smul_eq_mul]
        have h2_fac : (Nat.factorization 2) p = 0 := by
          rw [Nat.Prime.factorization Nat.prime_two]
          exact Finsupp.single_eq_of_ne h2
        rw [h2_fac, mul_zero, zero_add]
      rwa [←hn_eq_fac]
    rcases odd_even_factorization_is_square u hu_pos h_u_even with ⟨w, hw⟩
    cases Nat.even_or_odd e with
    | inl he_even =>
      rcases he_even with ⟨c, hc⟩
      left
      use 2 ^ c * w
      have hcc : c + c = c * 2 := by omega
      calc n = 2 ^ e * u := hn_eq
           _ = 2 ^ (c + c) * w ^ 2 := by rw [hc, hw]
           _ = 2 ^ (c * 2) * w ^ 2 := by rw [hcc]
           _ = (2 ^ c) ^ 2 * w ^ 2 := by rw [←pow_mul]
           _ = (2 ^ c * w) ^ 2 := by ring
    | inr he_odd =>
      rcases he_odd with ⟨c, hc⟩
      right
      use 2 ^ c * w
      calc n = 2 ^ e * u := hn_eq
           _ = 2 ^ (2 * c + 1) * w ^ 2 := by rw [hc, hw]
           _ = 2 * (2 ^ (c * 2)) * w ^ 2 := by ring_nf
           _ = 2 * (2 ^ c) ^ 2 * w ^ 2 := by rw [←pow_mul]
           _ = 2 * (2 ^ c * w) ^ 2 := by ring
  · rintro (⟨m, rfl⟩ | ⟨m, rfl⟩)
    · intro p hp hp2
      have hm : m ≠ 0 := by rintro rfl; simp at hn
      rw [Nat.factorization_pow]
      simp only [Finsupp.coe_smul, Pi.smul_apply, smul_eq_mul]
      exact even_two_mul (m.factorization p)
    · intro p hp hp2
      have hm : m ≠ 0 := by rintro rfl; simp at hn
      have h2 : 2 ≠ 0 := by decide
      have hp_prime : p.Prime := Nat.prime_of_mem_primeFactors hp
      rw [Nat.factorization_mul h2 (pow_ne_zero 2 hm)]
      simp only [Finsupp.coe_add, Pi.add_apply, Nat.factorization_pow, Finsupp.coe_smul, Pi.smul_apply, smul_eq_mul]
      have h2_fac : (Nat.factorization 2) p = 0 := by
        rw [Nat.Prime.factorization Nat.prime_two]
        exact Finsupp.single_eq_of_ne hp2
      rw [h2_fac, zero_add]
      exact even_two_mul _

/-- A known theorem in number theory: σ(n) is odd iff n is a perfect square or twice a perfect square. -/
lemma odd_sigma_iff_square_or_double_square (n : ℕ) (hn : n > 0) :
  Odd (sigma n) ↔ (∃ m : ℕ, n = m ^ 2) ∨ (∃ m : ℕ, n = 2 * m ^ 2) := by
  have hn_ne : n ≠ 0 := Nat.pos_iff_ne_zero.mp hn
  rw [odd_sigma_iff_factorization n hn_ne]
  exact factorization_even_iff_square_or_double_square n hn_ne

lemma mod_four_cases (x : ℕ) : x % 4 = 0 ∨ x % 4 = 1 ∨ x % 4 = 2 ∨ x % 4 = 3 := by
  have _h_lt : x % 4 < 4 := Nat.mod_lt x (by decide)
  match h_eq : x % 4 with
  | 0 => exact Or.inl rfl
  | 1 => exact Or.inr (Or.inl rfl)
  | 2 => exact Or.inr (Or.inr (Or.inl rfl))
  | 3 => exact Or.inr (Or.inr (Or.inr rfl))
  | k + 4 =>
    have h_contra : False := by omega
    exact False.elim h_contra

lemma mod_four_eq_three_has_prime_factor (n : ℕ) (h_pos : n > 0) (h_mod : n % 4 = 3) :
  ∃ q : ℕ, q.Prime ∧ q ∣ n ∧ q % 4 = 3 := by
  revert h_pos h_mod
  induction n using Nat.strong_induction_on with
  | h m ih =>
    intro h_pos h_mod
    by_cases hm : m.Prime
    · exact ⟨m, hm, dvd_rfl, h_mod⟩
    · have hm2 : m ≥ 2 := by omega
      obtain ⟨a, h_dvd, ha1, ham⟩ := Nat.exists_dvd_of_not_prime2 hm2 hm
      obtain ⟨b, hb_eq⟩ := h_dvd
      have _hb1 : 1 < b := by
        by_contra h_contra
        have _h_le : b ≤ 1 := by omega
        rcases Nat.eq_zero_or_pos b with rfl | hb_pos2
        · rw [mul_zero] at hb_eq; omega
        · have h_b_one : b = 1 := by omega
          rw [h_b_one, mul_one] at hb_eq
          omega
      have hbm : b < m := by
        have h_mul_ge : a * b ≥ 2 * b := Nat.mul_le_mul_right b ha1
        have _h_2b : m ≥ 2 * b := by rw [hb_eq]; exact h_mul_ge
        have _h_b_pos : b > 0 := by omega
        omega
      have ha_pos : a > 0 := by omega
      have hb_pos : b > 0 := by omega

      have h_mod_ab : (a * b) % 4 = 3 := by rw [←hb_eq]; exact h_mod

      have ha_mod : a % 4 = 1 ∨ a % 4 = 3 := by
        have ha_rem := mod_four_cases a
        rcases ha_rem with h0 | h1 | h2 | h3
        · have h_zero : (a * b) % 4 = 0 := by
            calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                 _ = (0 * (b % 4)) % 4 := by rw [h0]
                 _ = 0 := by simp
          rw [h_zero] at h_mod_ab
          contradiction
        · left; exact h1
        · have h_zero_two : (a * b) % 4 = 0 ∨ (a * b) % 4 = 2 := by
            have hb_rem := mod_four_cases b
            rcases hb_rem with hr0 | hr1 | hr2 | hr3
            · left
              calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                   _ = (2 * 0) % 4 := by rw [h2, hr0]
                   _ = 0 := by decide
            · right
              calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                   _ = (2 * 1) % 4 := by rw [h2, hr1]
                   _ = 2 := by decide
            · left
              calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                   _ = (2 * 2) % 4 := by rw [h2, hr2]
                   _ = 0 := by decide
            · right
              calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                   _ = (2 * 3) % 4 := by rw [h2, hr3]
                   _ = 2 := by decide
          rcases h_zero_two with hz0 | hz2
          · rw [hz0] at h_mod_ab; contradiction
          · rw [hz2] at h_mod_ab; contradiction
        · right; exact h3

      have hb_mod : b % 4 = 1 ∨ b % 4 = 3 := by
        have hb_rem := mod_four_cases b
        rcases hb_rem with h0 | h1 | h2 | h3
        · have h_zero : (a * b) % 4 = 0 := by
            calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                 _ = ((a % 4) * 0) % 4 := by rw [h0]
                 _ = 0 := by simp
          rw [h_zero] at h_mod_ab
          contradiction
        · left; exact h1
        · have h_zero_two : (a * b) % 4 = 0 ∨ (a * b) % 4 = 2 := by
            have ha_rem := mod_four_cases a
            rcases ha_rem with hr0 | hr1 | hr2 | hr3
            · left
              calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                   _ = (0 * 2) % 4 := by rw [hr0, h2]
                   _ = 0 := by decide
            · right
              calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                   _ = (1 * 2) % 4 := by rw [hr1, h2]
                   _ = 2 := by decide
            · left
              calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                   _ = (2 * 2) % 4 := by rw [hr2, h2]
                   _ = 0 := by decide
            · right
              calc (a * b) % 4 = ((a % 4) * (b % 4)) % 4 := Nat.mul_mod a b 4
                   _ = (3 * 2) % 4 := by rw [hr3, h2]
                   _ = 2 := by decide
          rcases h_zero_two with hz0 | hz2
          · rw [hz0] at h_mod_ab; contradiction
          · rw [hz2] at h_mod_ab; contradiction
        · right; exact h3

      rcases ha_mod with ha1_mod | ha3_mod
      · rcases hb_mod with hb1_mod | hb3_mod
        · have h_contra : (a * b) % 4 = 1 := by
            calc (a * b) % 4 = (a % 4 * (b % 4)) % 4 := Nat.mul_mod a b 4
                 _ = (1 * 1) % 4 := by rw [ha1_mod, hb1_mod]
                 _ = 1 := by decide
          rw [h_contra] at h_mod_ab
          contradiction
        · rcases ih b hbm hb_pos hb3_mod with ⟨q, hq_prime, hq_dvd, hq_mod⟩
          exact ⟨q, hq_prime, dvd_trans hq_dvd ⟨a, by rw [hb_eq, mul_comm]⟩, hq_mod⟩
      · rcases ih a ham ha_pos ha3_mod with ⟨q, hq_prime, hq_dvd, hq_mod⟩
        exact ⟨q, hq_prime, dvd_trans hq_dvd ⟨b, hb_eq⟩, hq_mod⟩

lemma two_pow_sub_one_mod_four {k : ℕ} (hk : k ≥ 2) : (2 ^ k - 1) % 4 = 3 := by
  have h_pow : 2 ^ k = 4 * 2 ^ (k - 2) := by
    calc 2 ^ k = 2 ^ (k - 2 + 2) := by congr 1; omega
         _ = 2 ^ (k - 2) * 2 ^ 2 := by rw [pow_add]
         _ = 4 * 2 ^ (k - 2) := by ring
  have _h_pow_pos : 2 ^ (k - 2) ≥ 1 := Nat.one_le_pow' (k - 2) 1
  have h_pow_sub : 2 ^ k - 1 = 4 * (2 ^ (k - 2) - 1) + 3 := by omega
  rw [h_pow_sub]
  have _h_mod_add : (4 * (2 ^ (k - 2) - 1) + 3) % 4 = (4 * (2 ^ (k - 2) - 1) % 4 + 3 % 4) % 4 := Nat.add_mod (4 * (2 ^ (k - 2) - 1)) 3 4
  have _h_mod_mul : 4 * (2 ^ (k - 2) - 1) % 4 = 0 := Nat.mul_mod_right 4 _
  have _h_mod_three : 3 % 4 = 3 := by decide
  omega

lemma sigma_two_pow_eq_sum (k : ℕ) : sigma (2 ^ k) = ∑ x ∈ range (k + 1), 2 ^ x := by
  unfold sigma
  exact sum_divisors_prime_pow Nat.prime_two

lemma geom_sum_two_eq (k : ℕ) : (∑ x ∈ range (k + 1), 2 ^ x) = 2 ^ (k + 1) - 1 := by
  induction k with
  | zero => simp
  | succ k ih =>
    rw [sum_range_succ, ih]
    have h_pow : 2 ^ (k + 1) ≥ 1 := Nat.one_le_pow' (k + 1) 1
    have _h_pow2 : 2 ^ (k + 1 + 1) = 2 * 2 ^ (k + 1) := by ring
    omega

lemma coprime_two_pow_sq_odd_even (e u : ℕ) (hu : ¬ 2 ∣ u) : Nat.Coprime (2 ^ (2 * e)) (u ^ 2) := by
  have hc : Nat.Coprime 2 u := (Nat.Prime.coprime_iff_not_dvd Nat.prime_two).mpr hu
  exact Nat.Coprime.pow_left (2 * e) (Nat.Coprime.pow_right 2 hc)

lemma qpn_sq_divisibility (e u : ℕ) (hu : ¬ 2 ∣ u) (h_eq : sigma (2 ^ (2 * e) * u ^ 2) = 2 * (2 ^ (2 * e) * u ^ 2) + 1) :
  (2 ^ (2 * e + 1) - 1) ∣ (u ^ 2 + 1) := by
  have hc := coprime_two_pow_sq_odd_even e u hu
  have h_sigma : sigma (2 ^ (2 * e) * u ^ 2) = sigma (2 ^ (2 * e)) * sigma (u ^ 2) := by
    unfold sigma
    exact Nat.Coprime.sum_divisors_mul hc
  rw [sigma_two_pow_eq_sum, geom_sum_two_eq] at h_sigma
  have h_sub_sigma : (2 ^ (2 * e + 1) - 1) * sigma (u ^ 2) = 2 * (2 ^ (2 * e) * u ^ 2) + 1 := by
    calc (2 ^ (2 * e + 1) - 1) * sigma (u ^ 2) = sigma (2 ^ (2 * e) * u ^ 2) := h_sigma.symm
         _ = 2 * (2 ^ (2 * e) * u ^ 2) + 1 := h_eq
  have h_expand : 2 * (2 ^ (2 * e) * u ^ 2) = 2 ^ (2 * e + 1) * u ^ 2 := by ring
  rw [h_expand] at h_sub_sigma

  set M := 2 ^ (2 * e + 1) - 1
  have hM_add : M + 1 = 2 ^ (2 * e + 1) := by
    have _hM_pos : 2 ^ (2 * e + 1) ≥ 1 := Nat.one_le_pow' (2 * e + 1) 1
    omega

  have _h_alg : M * sigma (u ^ 2) = M * u ^ 2 + (u ^ 2 + 1) := by
    calc M * sigma (u ^ 2) = 2 ^ (2 * e + 1) * u ^ 2 + 1 := h_sub_sigma
         _ = (M + 1) * u ^ 2 + 1 := by rw [←hM_add]
         _ = M * u ^ 2 + u ^ 2 + 1 := by ring

  have h_diff : u ^ 2 + 1 = M * (sigma (u ^ 2) - u ^ 2) := by
    rw [Nat.mul_sub_left_distrib]
    omega

  exact ⟨sigma (u ^ 2) - u ^ 2, h_diff⟩

lemma sum_range_add_two (n : ℕ) (f : ℕ → ℕ) :
  ∑ x ∈ range (n + 2), f x = (∑ x ∈ range n, f x) + f n + f (n + 1) := by
  rw [sum_range_succ, sum_range_succ, add_assoc]

lemma geom_sum_two_odd_mod_three (e : ℕ) : (∑ x ∈ range (2 * e + 2), 2 ^ x) % 3 = 0 := by
  induction e with
  | zero => rfl
  | succ e ih =>
    have h_rw : 2 * (e + 1) + 2 = (2 * e + 2) + 2 := by ring
    rw [h_rw, sum_range_add_two]
    have h_add : (∑ x ∈ range (2 * e + 2), 2 ^ x) + 2 ^ (2 * e + 2) + 2 ^ (2 * e + 2 + 1) =
      (∑ x ∈ range (2 * e + 2), 2 ^ x) + 3 * 2 ^ (2 * e + 2) := by ring
    rw [h_add, Nat.add_mod, ih]
    omega

lemma sigma_two_pow_odd_mod_three (e : ℕ) : (sigma (2 ^ (2 * e + 1))) % 3 = 0 := by
  rw [sigma_two_pow_eq_sum]
  exact geom_sum_two_odd_mod_three e

lemma coprime_two_pow_sq_odd (e u : ℕ) (hu : ¬ 2 ∣ u) : Nat.Coprime (2 ^ (2 * e + 1)) (u ^ 2) := by
  have hc : Nat.Coprime 2 u := (Nat.Prime.coprime_iff_not_dvd Nat.prime_two).mpr hu
  exact Nat.Coprime.pow_left (2 * e + 1) (Nat.Coprime.pow_right 2 hc)

lemma double_sq_sigma_mod_three (e u : ℕ) (hu : ¬ 2 ∣ u) : (sigma (2 * (2 ^ e * u) ^ 2)) % 3 = 0 := by
  have h_rw : 2 * (2 ^ e * u) ^ 2 = 2 ^ (2 * e + 1) * u ^ 2 := by
    calc 2 * (2 ^ e * u) ^ 2 = 2 * (2 ^ (2 * e) * u ^ 2) := by ring
      _ = 2 ^ (2 * e + 1) * u ^ 2 := by ring
  rw [h_rw]
  have hc := coprime_two_pow_sq_odd e u hu
  have h_sigma : sigma (2 ^ (2 * e + 1) * u ^ 2) = sigma (2 ^ (2 * e + 1)) * sigma (u ^ 2) := by
    unfold sigma
    exact Nat.Coprime.sum_divisors_mul hc
  rw [h_sigma]
  have h_mod := sigma_two_pow_odd_mod_three e
  rw [Nat.mul_mod, h_mod]
  omega

lemma four_sq_add_one_mod_three_ne_zero (m : ℕ) : (4 * m ^ 2 + 1) % 3 ≠ 0 := by
  have hm : m % 3 = 0 ∨ m % 3 = 1 ∨ m % 3 = 2 := by omega
  rcases hm with _h | _h | _h
  · have h_m : ∃ k, m = 3 * k := ⟨m / 3, by omega⟩
    rcases h_m with ⟨k, hk⟩
    have h_sq : 4 * m ^ 2 + 1 = 3 * (12 * k ^ 2) + 1 := by
      calc 4 * m ^ 2 + 1 = 4 * (3 * k) ^ 2 + 1 := by rw [hk]
      _ = 3 * (12 * k ^ 2) + 1 := by ring
    rw [h_sq]
    omega
  · have h_m : ∃ k, m = 3 * k + 1 := ⟨m / 3, by omega⟩
    rcases h_m with ⟨k, hk⟩
    have h_sq : 4 * m ^ 2 + 1 = 3 * (12 * k ^ 2 + 8 * k + 1) + 2 := by
      calc 4 * m ^ 2 + 1 = 4 * (3 * k + 1) ^ 2 + 1 := by rw [hk]
      _ = 3 * (12 * k ^ 2 + 8 * k + 1) + 2 := by ring
    rw [h_sq]
    omega
  · have h_m : ∃ k, m = 3 * k + 2 := ⟨m / 3, by omega⟩
    rcases h_m with ⟨k, hk⟩
    have h_sq : 4 * m ^ 2 + 1 = 3 * (12 * k ^ 2 + 16 * k + 5) + 2 := by
      calc 4 * m ^ 2 + 1 = 4 * (3 * k + 2) ^ 2 + 1 := by rw [hk]
      _ = 3 * (12 * k ^ 2 + 16 * k + 5) + 2 := by ring
    rw [h_sq]
    omega

/--
  Geometric sum identity in ℕ without truncated subtraction.
  For `p ≥ 1`: `(p - 1) * (∑ᵢ₌₀ⁿ⁻¹ pⁱ) + 1 = pⁿ`.
  This formulation avoids the subtraction `pⁿ - 1` which is
  problematic in ℕ, and is used to connect Zsigmondy divisibility
  hypotheses to the sum-of-divisors function.
-/
lemma nat_geom_sum (p n : ℕ) (hp : 1 ≤ p) :
    (p - 1) * (∑ i ∈ Finset.range n, p ^ i) + 1 = p ^ n := by
  induction n with
  | zero => simp
  | succ n ih =>
    rw [Finset.sum_range_succ, mul_add]
    have h1 : (p - 1) * (∑ i ∈ Finset.range n, p ^ i) + (p - 1) * p ^ n + 1 =
              ((p - 1) * (∑ i ∈ Finset.range n, p ^ i) + 1) + (p - 1) * p ^ n := by omega
    rw [h1, ih]
    have h2 : p ^ n + (p - 1) * p ^ n = p * p ^ n := by
      have h_add : 1 * p ^ n + (p - 1) * p ^ n = (1 + (p - 1)) * p ^ n :=
        (add_mul 1 (p - 1) (p ^ n)).symm
      have h_one : 1 * p ^ n = p ^ n := one_mul (p ^ n)
      have h_p : 1 + (p - 1) = p := by omega
      rw [h_p] at h_add
      rw [h_one] at h_add
      exact h_add
    rw [h2]
    have h3 : p * p ^ n = p ^ (n + 1) := by
      have h_pow_add : p ^ 1 * p ^ n = p ^ (1 + n) := (pow_add p 1 n).symm
      have h_pow_one : p ^ 1 = p := pow_one p
      have h_add : 1 + n = n + 1 := by omega
      rw [h_pow_one] at h_pow_add
      rw [h_add] at h_pow_add
      exact h_pow_add
    exact h3

/-- Product lower bound: for `x, y ≥ 2`, `x + y ≤ x * y`. -/
lemma lemma_x_y_ge_x_add_y {x y : ℕ} (hx : 2 ≤ x) (hy : 2 ≤ y) : x + y ≤ x * y := by
  nlinarith

theorem lemma_divisibility_transitive (n d c : ℕ) (hc : 0 < c) (hd : 0 < d) (hnd : d ∣ n) (hdc : c ∣ d) :
    c ∣ n :=
  dvd_trans hdc hnd

theorem lemma_smallest_factor_is_prime (n d : ℕ) (hn : 1 < n) (hd : 1 < d) (hdvd : d ∣ n)
    (hmin : ∀ c, 1 < c ∧ c < d → ¬ c ∣ n) : d.Prime := by
  refine Nat.prime_def_minFac.mpr ⟨hd, ?_⟩
  have h_minFac_dvd : d.minFac ∣ d := Nat.minFac_dvd d
  have h_minFac_le : d.minFac ≤ d := Nat.le_of_dvd (by omega) h_minFac_dvd
  have h_minFac_prime : d.minFac.Prime := Nat.minFac_prime (by omega)
  have h_minFac_gt1 : 1 < d.minFac := h_minFac_prime.one_lt
  by_cases h_eq : d.minFac = d
  · exact h_eq.symm
  · have h_lt : d.minFac < d := lt_of_le_of_ne h_minFac_le h_eq
    have h_dvd_n : d.minFac ∣ n := dvd_trans h_minFac_dvd hdvd
    exfalso
    exact hmin d.minFac ⟨h_minFac_gt1, h_lt⟩ h_dvd_n

theorem lemma_composite_has_prime_factor_le_sqrt (n : ℕ) (hn : 1 < n) (hnot : ¬ n.Prime) :
    ∃ p, p.Prime ∧ p ∣ n ∧ p * p ≤ n :=
  ⟨n.minFac, Nat.minFac_prime (by omega), Nat.minFac_dvd n, Nat.minFac_sq_le_of_not_prime hn hnot⟩

theorem lemma_modpow_mod_divisibility (a m n p : ℕ) (hn : 0 < n) (hp : 1 < p) (hdiv : p ∣ n) (hpow : a ^ m % n = 1) :
    a ^ m % p = 1 := by
  have h_dvd : n ∣ a ^ m - 1 := by
    have := Nat.div_add_mod (a ^ m) n
    rw [hpow] at this
    have : a ^ m - 1 = n * (a ^ m / n) := by omega
    exact ⟨a ^ m / n, this⟩
  have h_pdvd : p ∣ a ^ m - 1 := dvd_trans hdiv h_dvd
  obtain ⟨k, hk⟩ := h_pdvd
  have h_eq : a ^ m = p * k + 1 := by omega
  rw [h_eq, Nat.add_mod, Nat.mul_mod_right, zero_add, Nat.mod_eq_of_lt (by omega)]

theorem lemma_modpow_add_mul (a q d r p : ℕ) (hp : 1 < p) (hpow : a ^ d % p = 1) :
    a ^ (q * d + r) % p = a ^ r % p := by
  rw [pow_add, pow_mul]
  have h_mod : (a ^ d) ^ q % p = 1 := by
    induction q with
    | zero => simp [Nat.mod_eq_of_lt hp]
    | succ q ih =>
      rw [pow_succ, Nat.mul_mod, ih, hpow]
      simp
  rw [Nat.mul_mod, h_mod, one_mul, Nat.mod_mod]

theorem lemma_order_exists (a m p : ℕ) (hp : 1 < p) (hm : 0 < m) (hpow : a ^ m % p = 1) :
    ∃ d, 0 < d ∧ a ^ d % p = 1 ∧ (∀ k, 0 < k ∧ k < d → a ^ k % p ≠ 1) ∧ d ∣ m := by
  have h_ex : ∃ k, 0 < k ∧ a ^ k % p = 1 := ⟨m, hm, hpow⟩
  set d := Nat.find h_ex
  have hd_prop := Nat.find_spec h_ex
  have hd_min := fun k hk => Nat.find_min h_ex hk
  have hd_pos : 0 < d := hd_prop.1
  have hd_pow : a ^ d % p = 1 := hd_prop.2
  have hd_least : ∀ k, 0 < k ∧ k < d → a ^ k % p ≠ 1 := by
    intro k hk
    by_contra h_contra
    have := hd_min k hk.2
    push_neg at this
    exact this hk.1 h_contra
  have hd_dvd : d ∣ m := by
    by_contra h_not_dvd
    have h_div := Nat.div_add_mod m d
    set r := m % d
    have hr_pos : 0 < r := Nat.pos_of_ne_zero (by intro h0; rw [h0] at h_not_dvd; exact h_not_dvd (dvd_mul_right d (m / d)))
    have hr_lt : r < d := Nat.mod_lt m hd_pos
    have h_r_pow : a ^ r % p = 1 := by
      have := lemma_modpow_add_mul a (m / d) d r p hp hd_pow
      rw [←h_div] at this
      rw [←this, hpow]
    have := hd_least r ⟨hr_pos, hr_lt⟩
    exact this h_r_pow
  exact ⟨d, hd_pos, hd_pow, hd_least, hd_dvd⟩

theorem lemma_order_prime_factor (d f r_val p : ℕ) (hd : 0 < d) (hf : 0 < f) (hr : 0 < r_val)
    (h_dvd : d ∣ f * r_val) (h_not : ∀ q, q.Prime ∧ q ∣ f → ¬ (d ∣ (f * r_val) / q)) : f ∣ d := by
  have h_gcd_dvd : d ∣ f * r_val := h_dvd
  have h_f_dvd : f ∣ d := by
    by_contra h_not_dvd
    set g := Nat.gcd f d
    have hg_dvd_f : g ∣ f := Nat.gcd_dvd_left f d
    have hg_dvd_d : g ∣ d := Nat.gcd_dvd_right f d
    have h_f_div_g : 1 < f / g := by
      have h_le : g ≤ f := Nat.le_of_dvd hf hg_dvd_f
      have h_ne : g ≠ f := by
        intro h_eq
        rw [h_eq] at hg_dvd_d
        exact h_not_dvd hg_dvd_d
      have h_lt : g < f := lt_of_le_of_ne h_le h_ne
      exact Nat.one_lt_div_iff.mpr ⟨Nat.gcd_pos_of_pos_left d hf, h_lt⟩
    obtain ⟨q, hq_prime, hq_dvd⟩ := Nat.exists_prime_and_dvd (ne_of_gt h_f_div_g)
    have hq_dvd_f : q ∣ f := dvd_trans hq_dvd (Nat.div_dvd_of_dvd hg_dvd_f)
    have h_dvd_div : d ∣ (f * r_val) / q := by
      obtain ⟨k, hk⟩ := h_gcd_dvd
      obtain ⟨m, hm⟩ := hq_dvd_f
      have h_q_pos : 0 < q := hq_prime.pos
      have h_fr_q : (f * r_val) / q = m * r_val := by
        rw [hm, mul_assoc, Nat.mul_div_cancel_left _ h_q_pos]
      rw [h_fr_q]
      have hg_pos : 0 < g := Nat.gcd_pos_of_pos_left d hf
      have hq_dvd_fg : q ∣ f / g := hq_dvd
      obtain ⟨u, hu⟩ := hq_dvd_fg
      have h_fg_eq : f / g = q * u := hu
      have h_f_eq : f = g * (q * u) := by
        calc f = g * (f / g) := (Nat.gcd_mul_div_self hg_dvd_f).symm
             _ = g * (q * u) := by rw [h_fg_eq]
      have h_d_eq : d = g * (d / g) := (Nat.gcd_mul_div_self hg_dvd_d).symm
      rw [h_f_eq] at hk
      have h_k_eq : g * (q * u) * r_val = g * (d / g) * k := by
        rw [←hk, h_d_eq]
      have h_k_cancel : q * u * r_val = (d / g) * k := by
        have := Nat.eq_of_mul_eq_mul_left hg_pos (by calc g * (q * u * r_val) = g * (q * u) * r_val := by ring
                                                    _ = g * (d / g) * k := h_k_eq
                                                    _ = g * ((d / g) * k) := by ring)
        exact this
      have h_m_eq : m = g * u := by
        have h1 : f = q * m := hm
        have h2 : f = q * (g * u) := by calc f = g * (q * u) := h_f_eq _ = q * (g * u) := by ring
        exact Nat.eq_of_mul_eq_mul_left h_q_pos (h1.trans h2.symm)
      rw [h_m_eq]
      have h_m_r : g * u * r_val = g * (u * r_val) := by ring
      rw [h_m_r, h_d_eq]
      have h_dg_dvd : (d / g) ∣ u * r_val := by
        have h_coprime : Nat.Coprime (d / g) q := by
          have h_gcd1 : Nat.gcd (f / g) (d / g) = 1 := Nat.gcd_div_eq_one_of_dvd hg_dvd_f
          rw [h_fg_eq] at h_gcd1
          have := Nat.Coprime.of_mul_right_left (Nat.gcd_eq_one_iff_coprime.mp h_gcd1)
          exact this.symm
        have h_dg_dvd_qur : (d / g) ∣ q * (u * r_val) := by
          exact ⟨k, by calc q * (u * r_val) = q * u * r_val := by ring
                             _ = (d / g) * k := h_k_cancel⟩
        exact h_coprime.dvd_of_dvd_mul_left h_dg_dvd_qur
      obtain ⟨w, hw⟩ := h_dg_dvd
      use w
      calc g * (u * r_val) = g * ((d / g) * w) := by rw [hw]
           _ = g * (d / g) * w := by ring
    exact h_not q ⟨hq_prime, hq_dvd_f⟩ h_dvd_div
  exact h_f_dvd

theorem lemma_divisibility_bounds (a b : ℕ) (ha : 0 < a) (hb : 0 < b) (hdvd : a ∣ b) : a ≤ b :=
  Nat.le_of_dvd hb hdvd

theorem lemma_fermat_little_theorem (a p : ℕ) (hp : p.Prime) (hcop : ¬ p ∣ a) :
    a ^ (p - 1) % p = 1 := by
  have h_coprime : Nat.Coprime a p := (Nat.Prime.coprime_iff_not_dvd hp).mpr hcop
  have h_totient := Nat.modeq_pow_totient h_coprime
  rw [hp.totient] at h_totient
  exact h_totient

theorem lemma_order_le_p_minus_1 (a d p : ℕ) (hp : p.Prime) (hd_pos : 0 < d)
    (hd_least : ∀ k, 0 < k ∧ k < d → a ^ k % p ≠ 1) (hd_dvd : a ^ d % p = 1) :
    d ≤ p - 1 := by
  have h_cop : ¬ p ∣ a := by
    intro h_dvd
    have h_zero : a ^ d % p = 0 := by
      obtain ⟨k, hk⟩ := h_dvd
      rw [hk, mul_pow, Nat.mul_mod, zero_pow hd_pos.ne.symm, zero_mul, Nat.zero_mod]
    rw [h_zero] at hd_dvd
    contradiction
  have h_flt := lemma_fermat_little_theorem a p hp h_cop
  by_contra h_lt
  push_neg at h_lt
  have hp1_pos : 0 < p - 1 := Nat.sub_pos_of_lt hp.one_lt
  have := hd_least (p - 1) ⟨hp1_pos, h_lt⟩
  exact this h_flt

theorem lemma_square_comparison_contradiction (p f n : ℕ) (hp : p * p ≤ n) (hf : n - 1 < f * f) (hfp : f < p) : False := by
  have h1 : f * f < p * p := by nlinarith
  have h2 : f * f ≤ n - 1 := by omega
  omega

theorem lemma_f_squared_gt_n_minus_1 (f r_val n : ℕ) (h_eq : n - 1 = f * r_val) (h_gt : r_val < f) : n - 1 < f * f := by
  nlinarith

theorem lemma_pocklington_certificate (n a f r_val : ℕ) (hn : 1 < n) (h_eq : n - 1 = f * r_val) (h_gt : r_val < f)
    (h_pow : a ^ (n - 1) % n = 1)
    (h_gcd : ∀ q, q.Prime ∧ q ∣ f → a ^ ((n - 1) / q) % n ≠ 1 ∧ Nat.gcd (a ^ ((n - 1) / q) % n - 1) n = 1) :
    n.Prime := by
  by_contra h_not_prime
  obtain ⟨p, hp_prime, hp_dvd, hp_sq⟩ := lemma_composite_has_prime_factor_le_sqrt n hn h_not_prime
  have hp_gt1 : 1 < p := hp_prime.one_lt
  have h_pow_p : a ^ (n - 1) % p = 1 := lemma_modpow_mod_divisibility a (n - 1) n p (by omega) hp_gt1 hp_dvd h_pow
  have hn1_pos : 0 < n - 1 := by omega
  obtain ⟨d, hd_pos, hd_pow, hd_least, hd_dvd⟩ := lemma_order_exists a (n - 1) p hp_gt1 hn1_pos h_pow_p
  rw [h_eq] at hd_dvd
  have h_not_dvd_q : ∀ q, q.Prime ∧ q ∣ f → ¬ (d ∣ (f * r_val) / q) := by
    intro q hq hd_dvd_q
    have h_q_dvd_f : q ∣ f := hq.2
    have h_div_eq : (n - 1) / q = (f * r_val) / q := by rw [h_eq]
    have h_pow_div : a ^ ((n - 1) / q) % p = 1 := by
      rw [h_div_eq]
      obtain ⟨k, hk⟩ := hd_dvd_q
      have := lemma_modpow_add_mul a k d 0 p hp_gt1 hd_pow
      rw [add_zero] at this
      rw [←hk] at this
      rw [this]
      exact Nat.mod_eq_of_lt hp_gt1
    have h_p_dvd_sub : p ∣ a ^ ((n - 1) / q) % n - 1 := by
      have h_mod_mod : (a ^ ((n - 1) / q) % n) % p = 1 := by
        have h_div2 : p ∣ n := hp_dvd
        have := Nat.mod_mod_of_dvd (a ^ ((n - 1) / q)) h_div2
        rw [this]
        exact h_pow_div
      have h_mod_val : a ^ ((n - 1) / q) % n = p * ((a ^ ((n - 1) / q) % n) / p) + 1 := by
        have := Nat.div_add_mod (a ^ ((n - 1) / q) % n) p
        rw [h_mod_mod] at this
        omega
      use (a ^ ((n - 1) / q) % n) / p
      omega
    have h_gcd_spec := (h_gcd q hq).2
    have h_p_dvd_gcd : p ∣ Nat.gcd (a ^ ((n - 1) / q) % n - 1) n := Nat.dvd_gcd h_p_dvd_sub hp_dvd
    rw [h_gcd_spec] at h_p_dvd_gcd
    exact hp_prime.not_dvd_one h_p_dvd_gcd
  have h_hf_pos : 0 < f := by
    by_contra h0
    have : f = 0 := by omega
    rw [this, zero_mul] at h_eq
    omega
  have h_hr_pos : 0 < r_val := by
    by_contra h0
    have : r_val = 0 := by omega
    rw [this, mul_zero] at h_eq
    omega
  have h_f_dvd_d : f ∣ d := lemma_order_prime_factor d f r_val p hd_pos h_hf_pos h_hr_pos (by rwa [←h_eq]) h_not_dvd_q
  have h_f_le_d : f ≤ d := lemma_divisibility_bounds f d h_hf_pos hd_pos h_f_dvd_d
  have h_d_le_p1 : d ≤ p - 1 := lemma_order_le_p_minus_1 a d p hp_prime hd_pos hd_least hd_pow
  have h_f_lt_p : f < p := by omega
  have h_f_sq : n - 1 < f * f := lemma_f_squared_gt_n_minus_1 f r_val n h_eq h_gt
  exact lemma_square_comparison_contradiction p f n hp_sq h_f_sq h_f_lt_p

end UALBF.Pure.Arithmetic
