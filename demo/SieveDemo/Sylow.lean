import Mathlib.Algebra.Order.Archimedean.Basic
import Mathlib.Data.SetLike.Fintype
import Mathlib.GroupTheory.PGroup
import Mathlib.GroupTheory.NoncommPiCoprod
import Mathlib.Data.Fintype.Lattice

/-!
# The Sylow theorems, developed in the demo

This module deliberately does not import `Mathlib.GroupTheory.Sylow`.  It starts with
Mathlib's definitions of groups, subgroups, finite group actions and `p`-groups, defines
Sylow subgroups here, and proves existence, conjugacy, and the two usual counting
statements.  In particular, none of the results below is a forwarding wrapper around a
packaged Sylow theorem.
-/

namespace SieveDemo

open MulAction Subgroup
open scoped Pointwise

variable (p : ℕ) (G : Type*) [Group G]

/-- A Sylow `p`-subgroup is, by definition, a maximal `p`-subgroup. -/
structure Sylow extends Subgroup G where
  isPGroup' : IsPGroup p toSubgroup
  is_maximal' : ∀ {Q : Subgroup G}, IsPGroup p Q → toSubgroup ≤ Q → Q = toSubgroup

variable {p G}

namespace Sylow

attribute [coe] toSubgroup

instance : CoeOut (Sylow p G) (Subgroup G) := ⟨toSubgroup⟩

@[ext]
theorem ext {P Q : Sylow p G} (h : (P : Subgroup G) = Q) : P = Q := by
  cases P
  cases Q
  congr

instance : SetLike (Sylow p G) G where
  coe := (↑)
  coe_injective _ _ h := ext (SetLike.coe_injective h)

instance : PartialOrder (Sylow p G) := .ofSetLike (Sylow p G) G

instance : SubgroupClass (Sylow p G) G where
  mul_mem := Subgroup.mul_mem _
  one_mem _ := Subgroup.one_mem _
  inv_mem := Subgroup.inv_mem _

@[simp]
protected theorem coe_coe (P : Sylow p G) : (P : Subgroup G) = (P : Set G) := rfl

end Sylow

/-- Sylow I in its strongest elementary form: every `p`-subgroup lies in a
maximal `p`-subgroup.  The proof constructs the union of a chain and applies Zorn. -/
theorem IsPGroup.exists_le_sylow {P : Subgroup G} (hP : IsPGroup p P) :
    ∃ Q : Sylow p G, P ≤ Q :=
  Exists.elim
    (zorn_le_nonempty₀ { Q : Subgroup G | IsPGroup p Q }
      (fun c hc1 hc2 Q hQ =>
        ⟨{ carrier := ⋃ R : c, R,
            one_mem' := ⟨Q, ⟨⟨Q, hQ⟩, rfl⟩, Q.one_mem⟩,
            inv_mem' := fun {_} ⟨_, ⟨R, rfl⟩, hg⟩ => ⟨R, ⟨R, rfl⟩, R.1.inv_mem hg⟩,
            mul_mem' := fun {_} _ ⟨_, ⟨R, rfl⟩, hg⟩ ⟨_, ⟨S, rfl⟩, hh⟩ =>
              (hc2.total R.2 S.2).elim
                (fun T => ⟨S, ⟨S, rfl⟩, S.1.mul_mem (T hg) hh⟩)
                (fun T => ⟨R, ⟨R, rfl⟩, R.1.mul_mem hg (T hh)⟩) },
          fun ⟨g, _, ⟨S, rfl⟩, hg⟩ => by
            refine Exists.imp (fun k hk => ?_) (hc1 S.2 ⟨g, hg⟩)
            rw [Subtype.ext_iff, coe_pow] at hk ⊢
            exact hk,
          fun M hM _ hg => ⟨M, ⟨⟨M, hM⟩, rfl⟩, hg⟩⟩)
      P hP)
    fun {Q} h => ⟨⟨Q, h.2.prop, h.2.eq_of_ge⟩, h.1⟩

namespace Sylow

instance nonempty : Nonempty (Sylow p G) :=
  (SieveDemo.IsPGroup.exists_le_sylow (P := ⊥) IsPGroup.of_bot).nonempty

noncomputable instance inhabited : Inhabited (Sylow p G) :=
  Classical.inhabited_of_nonempty nonempty

/-- Conjugation preserves maximal `p`-subgroups. -/
instance pointwiseMulAction {A : Type*} [Group A] [MulDistribMulAction A G] :
    MulAction A (Sylow p G) where
  smul g P :=
    ⟨g • P.toSubgroup, P.2.map _, fun {Q} hQ hS =>
      inv_smul_eq_iff.mp
        (P.3 (hQ.map _) fun s hs =>
          (congr_arg (· ∈ g⁻¹ • Q) (inv_smul_smul g s)).mp
            (smul_mem_pointwise_smul (g • s) g⁻¹ Q
              (hS (smul_mem_pointwise_smul s g P hs))))⟩
  one_smul P := ext (one_smul A P.toSubgroup)
  mul_smul g h P := ext (mul_smul g h P.toSubgroup)

instance mulAction : MulAction G (Sylow p G) := compHom _ MulAut.conj

theorem smul_eq_iff_mem_normalizer {g : G} {P : Sylow p G} :
    g • P = P ↔ g ∈ normalizer P := by
  rw [eq_comm, SetLike.ext_iff, ← inv_mem_iff (G := G) (H := normalizer P),
    mem_set_normalizer_iff, inv_inv]
  exact forall_congr' fun h =>
    iff_congr Iff.rfl
      ⟨fun ⟨a, b, c⟩ => c ▸ by simpa [mul_assoc] using b,
       fun hh => ⟨(MulAut.conj g)⁻¹ h, hh, MulAut.apply_inv_self G (MulAut.conj g) h⟩⟩

end Sylow

theorem Subgroup.sylow_mem_fixedPoints_iff (H : Subgroup G) {P : Sylow p G} :
    P ∈ fixedPoints H (Sylow p G) ↔ H ≤ normalizer P := by
  simp_rw [IsConcreteLE.le_iff, ← Sylow.smul_eq_iff_mem_normalizer]
  exact Subtype.forall

theorem IsPGroup.inf_normalizer_sylow {P : Subgroup G} (hP : IsPGroup p P)
    (Q : Sylow p G) : P ⊓ normalizer Q = P ⊓ Q :=
  le_antisymm
    (le_inf inf_le_left
      (sup_eq_right.mp
        (Q.3 (hP.to_inf_left.to_sup_of_normal_right' Q.2 inf_le_right) le_sup_right)))
    (inf_le_inf_left P le_normalizer)

theorem IsPGroup.sylow_mem_fixedPoints_iff {P : Subgroup G} (hP : IsPGroup p P)
    {Q : Sylow p G} : Q ∈ fixedPoints P (Sylow p G) ↔ P ≤ Q := by
  rw [SieveDemo.Subgroup.sylow_mem_fixedPoints_iff P, ← inf_eq_left,
    SieveDemo.IsPGroup.inf_normalizer_sylow hP, inf_eq_left]

/-- Sylow II: all Sylow `p`-subgroups are conjugate. -/
instance Sylow.isPretransitive_of_finite [hp : Fact p.Prime]
    [Finite (Sylow p G)] : IsPretransitive G (Sylow p G) :=
  ⟨fun P Q => by
    have H := fun {R : Sylow p G} {S : orbit G P} =>
      calc
        S ∈ fixedPoints R (orbit G P) ↔ S.1 ∈ fixedPoints R (Sylow p G) :=
          forall_congr' fun _ => Subtype.ext_iff
        _ ↔ R.1 ≤ S := SieveDemo.IsPGroup.sylow_mem_fixedPoints_iff R.2
        _ ↔ S.1.1 = R := ⟨fun h => R.3 S.1.2 h, ge_of_eq⟩
    suffices Set.Nonempty (fixedPoints Q (orbit G P)) by
      exact Exists.elim this fun R hR => by
        rw [← Sylow.ext (H.mp hR)]
        exact R.2
    apply Q.2.nonempty_fixed_point_of_prime_not_dvd_card
    refine fun h => hp.out.not_dvd_one (Nat.modEq_zero_iff_dvd.mp ?_)
    calc
      1 = Nat.card (fixedPoints P (orbit G P)) := ?_
      _ ≡ Nat.card (orbit G P) [MOD p] :=
        (P.2.card_modEq_card_fixedPoints (orbit G P)).symm
      _ ≡ 0 [MOD p] := Nat.modEq_zero_iff_dvd.mpr h
    rw [← Nat.card_unique
      (α := ({⟨P, mem_orbit_self P⟩} : Set (orbit G P))), eq_comm]
    congr
    rw [Set.eq_singleton_iff_unique_mem]
    exact ⟨H.mpr rfl, fun R h => Subtype.ext (Sylow.ext (H.mp h))⟩⟩

variable (p G)

/-- Sylow III(a): the number of Sylow subgroups is `1` modulo `p`. -/
theorem card_sylow_modEq_one [Fact p.Prime] [Finite (Sylow p G)] :
    Nat.card (Sylow p G) ≡ 1 [MOD p] := by
  refine Sylow.nonempty.elim fun P : Sylow p G => ?_
  have hfixed : fixedPoints P.1 (Sylow p G) = {P} :=
    Set.ext fun Q : Sylow p G =>
      calc
        Q ∈ fixedPoints P (Sylow p G) ↔ P.1 ≤ Q :=
          SieveDemo.IsPGroup.sylow_mem_fixedPoints_iff P.2
        _ ↔ Q.1 = P.1 := ⟨P.3 Q.2, ge_of_eq⟩
        _ ↔ Q ∈ {P} := Sylow.ext_iff.symm.trans Set.mem_singleton_iff.symm
  have hcard : Nat.card (fixedPoints P.1 (Sylow p G)) = 1 := by simp [hfixed]
  exact (P.2.card_modEq_card_fixedPoints (Sylow p G)).trans (by rw [hcard])

variable {p G}

namespace Sylow

@[simp]
theorem orbit_eq_top [Fact p.Prime] [Finite (Sylow p G)] (P : Sylow p G) :
    orbit G P = ⊤ := top_le_iff.mp fun Q _ => exists_smul_eq G P Q

theorem stabilizer_eq_normalizer (P : Sylow p G) :
    stabilizer G P = normalizer P := by
  ext
  simp [smul_eq_iff_mem_normalizer]

/-- The conjugacy theorem identifies Sylow subgroups with cosets of a normalizer. -/
noncomputable def equivQuotientNormalizer [Fact p.Prime] [Finite (Sylow p G)]
    (P : Sylow p G) : Sylow p G ≃ G ⧸ normalizer P :=
  calc
    Sylow p G ≃ (⊤ : Set (Sylow p G)) := (Equiv.Set.univ (Sylow p G)).symm
    _ ≃ orbit G P := Set.equivOfEq P.orbit_eq_top.symm
    _ ≃ G ⧸ stabilizer G P := orbitEquivQuotientStabilizer G P
    _ ≃ G ⧸ normalizer P := by rw [P.stabilizer_eq_normalizer]

instance [Fact p.Prime] [Finite (Sylow p G)] (P : Sylow p G) :
    Finite (G ⧸ normalizer P) :=
  Finite.of_equiv (Sylow p G) P.equivQuotientNormalizer

/-- Sylow III(b): the number of Sylow subgroups is the normalizer index. -/
theorem card_eq_index_normalizer [Fact p.Prime] [Finite (Sylow p G)]
    (P : Sylow p G) : Nat.card (Sylow p G) = (normalizer (P : Set G)).index :=
  Nat.card_congr P.equivQuotientNormalizer

/-- Sylow III(c): the number of Sylow subgroups divides the subgroup index. -/
theorem card_dvd_index [Fact p.Prime] [Finite (Sylow p G)] (P : Sylow p G) :
    Nat.card (Sylow p G) ∣ P.index :=
  ((congr_arg _ P.card_eq_index_normalizer).mp dvd_rfl).trans
    (index_dvd_of_le le_normalizer)

end Sylow
end SieveDemo
