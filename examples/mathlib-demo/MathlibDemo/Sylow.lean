import Mathlib.GroupTheory.Sylow

/-!
  Demo copies of the core mathlib Sylow proof bodies.

  These theorems intentionally duplicate the shape of mathlib's proofs under
  local names so Sieve can extract the proof structure from this demo module.
-/

namespace MathlibDemo.Sylow

open MulAction Subgroup
open scoped Pointwise

variable {p : ℕ} {G : Type*} [Group G]

/-- Sylow I, in the stronger form that every `p`-subgroup lies in a Sylow subgroup. -/
theorem exists_le_sylow_demo {P : Subgroup G} (hP : IsPGroup p P) : ∃ Q : Sylow p G, P ≤ Q :=
  Exists.elim
    (zorn_le_nonempty₀ { Q : Subgroup G | IsPGroup p Q }
      (fun c hc1 hc2 Q hQ =>
        ⟨{ carrier := ⋃ R : c, R,
            one_mem' := ⟨Q, ⟨⟨Q, hQ⟩, rfl⟩, Q.one_mem⟩,
            inv_mem' := fun {_} ⟨_, ⟨R, rfl⟩, hg⟩ => ⟨R, ⟨R, rfl⟩, R.1.inv_mem hg⟩,
            mul_mem' := fun {_} _ ⟨_, ⟨R, rfl⟩, hg⟩ ⟨_, ⟨S, rfl⟩, hh⟩ =>
              (hc2.total R.2 S.2).elim
                (fun T => ⟨S, ⟨S, rfl⟩, S.1.mul_mem (T hg) hh⟩)
                fun T => ⟨R, ⟨R, rfl⟩, R.1.mul_mem hg (T hh)⟩ },
          fun ⟨g, _, ⟨S, rfl⟩, hg⟩ => by
            refine Exists.imp (fun k hk => ?_) (hc1 S.2 ⟨g, hg⟩)
            rw [Subtype.ext_iff, coe_pow] at hk ⊢
            assumption,
          fun M hM _ hg => ⟨M, ⟨⟨M, hM⟩, rfl⟩, hg⟩⟩)
      P hP)
    fun {Q} h => ⟨⟨Q, h.2.prop, h.2.eq_of_ge⟩, h.1⟩

/-- Sylow I: Sylow `p`-subgroups exist. -/
theorem sylow_exists_demo : Nonempty (Sylow p G) :=
  exists_le_sylow_demo IsPGroup.of_bot |>.nonempty

/-- Fixed-point characterization used by the Sylow conjugacy/counting proofs. -/
theorem sylow_mem_fixedPoints_iff_demo (H : Subgroup G) {P : Sylow p G} :
    P ∈ fixedPoints H (Sylow p G) ↔ H ≤ normalizer P := by
  simp_rw [IsConcreteLE.le_iff, ← Sylow.smul_eq_iff_mem_normalizer]
  exact Subtype.forall

/-- Intersecting a `p`-subgroup with the normalizer of a Sylow subgroup. -/
theorem inf_normalizer_sylow_demo {P : Subgroup G} (hP : IsPGroup p P) (Q : Sylow p G) :
    P ⊓ normalizer Q = P ⊓ Q :=
  le_antisymm
    (le_inf inf_le_left
      (sup_eq_right.mp
        (Q.3 (hP.to_inf_left.to_sup_of_normal_right' Q.2 inf_le_right) le_sup_right)))
    (inf_le_inf_left P le_normalizer)

/-- Fixed-point characterization specialized to `p`-subgroups. -/
theorem isPGroup_sylow_mem_fixedPoints_iff_demo
    {P : Subgroup G} (hP : IsPGroup p P) {Q : Sylow p G} :
    Q ∈ fixedPoints P (Sylow p G) ↔ P ≤ Q := by
  rw [sylow_mem_fixedPoints_iff_demo P, ← inf_eq_left,
    inf_normalizer_sylow_demo hP, inf_eq_left]

/-- Sylow II: all Sylow `p`-subgroups are conjugate. -/
instance sylow_isPretransitive_of_finite_demo [hp : Fact p.Prime] [Finite (Sylow p G)] :
    MulAction.IsPretransitive G (Sylow p G) :=
  ⟨fun P Q => by
    have H := fun {R : Sylow p G} {S : orbit G P} =>
      calc
        S ∈ fixedPoints R (orbit G P) ↔ S.1 ∈ fixedPoints R (Sylow p G) :=
          forall_congr' fun _a => Subtype.ext_iff
        _ ↔ R.1 ≤ S := isPGroup_sylow_mem_fixedPoints_iff_demo R.2
        _ ↔ S.1.1 = R := ⟨fun h => R.3 S.1.2 h, ge_of_eq⟩
    suffices Set.Nonempty (fixedPoints Q (orbit G P)) by
      exact Exists.elim this fun R hR => by
        rw [← Sylow.ext (H.mp hR)]
        exact R.2
    apply Q.2.nonempty_fixed_point_of_prime_not_dvd_card
    refine fun h => hp.out.not_dvd_one (Nat.modEq_zero_iff_dvd.mp ?_)
    calc
      1 = Nat.card (fixedPoints P (orbit G P)) := ?_
      _ ≡ Nat.card (orbit G P) [MOD p] := (P.2.card_modEq_card_fixedPoints (orbit G P)).symm
      _ ≡ 0 [MOD p] := Nat.modEq_zero_iff_dvd.mpr h
    rw [← Nat.card_unique (α := ({⟨P, mem_orbit_self P⟩} : Set (orbit G P))), eq_comm]
    congr
    rw [Set.eq_singleton_iff_unique_mem]
    exact ⟨H.mpr rfl, fun R h => Subtype.ext (Sylow.ext (H.mp h))⟩⟩

/-- Sylow III: the number of Sylow `p`-subgroups is congruent to `1` modulo `p`. -/
theorem card_sylow_modEq_one_demo (p : ℕ) (G : Type*) [Group G]
    [Fact p.Prime] [Finite (Sylow p G)] :
    Nat.card (Sylow p G) ≡ 1 [MOD p] := by
  refine sylow_exists_demo.elim fun P : Sylow p G => ?_
  have : fixedPoints P.1 (Sylow p G) = {P} :=
    Set.ext fun Q : Sylow p G =>
      calc
        Q ∈ fixedPoints P (Sylow p G) ↔ P.1 ≤ Q :=
          isPGroup_sylow_mem_fixedPoints_iff_demo P.2
        _ ↔ Q.1 = P.1 := ⟨P.3 Q.2, ge_of_eq⟩
        _ ↔ Q ∈ {P} := Sylow.ext_iff.symm.trans Set.mem_singleton_iff.symm
  have : Nat.card (fixedPoints P.1 (Sylow p G)) = 1 := by
    simp [this]
  exact (P.2.card_modEq_card_fixedPoints (Sylow p G)).trans (by rw [this])

end MathlibDemo.Sylow
