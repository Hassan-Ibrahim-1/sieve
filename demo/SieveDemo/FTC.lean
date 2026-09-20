import Mathlib.Analysis.Calculus.Deriv.Slope
import Mathlib.Analysis.Calculus.FDeriv.Measurable
import Mathlib.MeasureTheory.Integral.Bochner.VitaliCaratheodory
import Mathlib.MeasureTheory.Integral.DominatedConvergence
import Mathlib.Analysis.Calculus.TangentCone.Prod

/-!
# A from-definitions Fundamental Theorem of Calculus

This module intentionally does not import Mathlib's interval FTC module.  It gives local
definitions of limit, derivative, and oriented interval integral.  The proof below is the
hard real-valued FTC-2 argument: it obtains an upper semicontinuous majorant using
Vitali--Carathéodory, propagates a local derivative inequality across the interval, proves
both inequalities, and concludes by antisymmetry.

The primitive integral used in `Integral` is Mathlib's Bochner integral.  Its construction
from simple functions is below the boundary of this demo, just as real-number completeness
and finite-set cardinality are below the boundary of the Sylow demo.  The oriented interval
integral itself is defined here rather than imported from an FTC theorem.
-/

noncomputable section

namespace SieveDemo.Calculus

open MeasureTheory Set Filter Function Asymptotics
open scoped Topology ENNReal Interval NNReal

/-- `f` tends to `L` at `x` within `s`, directly in terms of filters. -/
def HasLimitWithinAt (f : ℝ → ℝ) (L : ℝ) (s : Set ℝ) (x : ℝ) : Prop :=
  Tendsto f (nhdsWithin x s) (nhds L)

/-- `f` tends to `L` at `x`, directly in terms of neighborhood filters. -/
def HasLimitAt (f : ℝ → ℝ) (L x : ℝ) : Prop :=
  Tendsto f (nhds x) (nhds L)

/-- A punctured limit excludes the point at which the limit is taken. -/
def HasPuncturedLimitAt (f : ℝ → ℝ) (L x : ℝ) : Prop :=
  HasLimitWithinAt f L ({x}ᶜ) x

/-- The derivative is the punctured-neighborhood limit of difference quotients. -/
def HasDerivativeAt (f : ℝ → ℝ) (f' x : ℝ) : Prop :=
  HasPuncturedLimitAt (fun h => h⁻¹ * (f (x + h) - f x)) f' 0

/-- Continuity on a set, defined pointwise using within-limits. -/
def IsContinuousOn (f : ℝ → ℝ) (s : Set ℝ) : Prop :=
  ∀ x ∈ s, HasLimitWithinAt f (f x) s x

/-- Integrability on a closed interval; this exposes the exact analytic hypothesis
used by the theorem while keeping the integral itself total. -/
def IsIntegrableOn (f : ℝ → ℝ) (a b : ℝ) : Prop :=
  MeasureTheory.IntegrableOn f (Icc a b)

/-- A set integral, based on the primitive Bochner integral of a restricted measure. -/
def SetIntegral (f : ℝ → ℝ) (s : Set ℝ) : ℝ :=
  ∫ x in s, f x

/-- The oriented integral is the forward `Ioc` integral minus the reverse one. -/
def Integral (f : ℝ → ℝ) (a b : ℝ) : ℝ :=
  SetIntegral f (Ioc a b) - SetIntegral f (Ioc b a)

theorem hasDerivativeAt_iff_hasDerivAt {f : ℝ → ℝ} {f' x : ℝ} :
    HasDerivativeAt f f' x ↔ HasDerivAt f f' x := by
  rw [HasDerivativeAt, HasPuncturedLimitAt, HasLimitWithinAt,
    hasDerivAt_iff_tendsto_slope_zero]
  simp only [smul_eq_mul]

theorem isContinuousOn_iff {f : ℝ → ℝ} {s : Set ℝ} :
    IsContinuousOn f s ↔ ContinuousOn f s := Iff.rfl

theorem isIntegrableOn_iff {f : ℝ → ℝ} {a b : ℝ} :
    IsIntegrableOn f a b ↔ MeasureTheory.IntegrableOn f (Icc a b) := Iff.rfl

theorem integral_eq_intervalIntegral (f : ℝ → ℝ) (a b : ℝ) :
    Integral f a b = ∫ x in a..b, f x := by
  rfl

section FTC2

variable {g' g φ : ℝ → ℝ} {a b : ℝ}

/-- The propagation step in FTC-2.  A pointwise upper bound on the right
derivative bounds the total increment. -/
theorem sub_le_integral_of_hasDeriv_right_of_le_Ico
    (hab : a ≤ b)
    (hcont : ContinuousOn g (Icc a b))
    (hderiv : ∀ x ∈ Ico a b, HasDerivWithinAt g (g' x) (Ioi x) x)
    (φint : IntegrableOn φ (Icc a b))
    (hφg : ∀ x ∈ Ico a b, g' x ≤ φ x) :
    g b - g a ≤ ∫ y in a..b, φ y := by
  refine le_of_forall_pos_le_add fun ε εpos => ?_
  rcases exists_lt_lowerSemicontinuous_integral_lt φ φint εpos with
    ⟨G', f_lt_G', G'cont, G'int, G'lt_top, hG'⟩
  set s := {t | g t - g a ≤ ∫ u in a..t, (G' u).toReal} ∩ Icc a b
  have s_closed : IsClosed s := by
    have hc : ContinuousOn
        (fun t => (g t - g a, ∫ u in a..t, (G' u).toReal)) (Icc a b) := by
      rw [← uIcc_of_le hab] at G'int hcont ⊢
      exact (hcont.sub continuousOn_const).prodMk
        (intervalIntegral.continuousOn_primitive_interval G'int)
    simp only [s, inter_comm]
    exact hc.preimage_isClosed_of_isClosed isClosed_Icc
      OrderClosedTopology.isClosed_le'
  have main : Icc a b ⊆ {t | g t - g a ≤ ∫ u in a..t, (G' u).toReal} := by
    refine s_closed.Icc_subset_of_forall_exists_gt
      (by simp only [intervalIntegral.integral_same, mem_ofPred_eq, sub_self, le_rfl])
      fun t ht v t_lt_v => ?_
    obtain ⟨y, g'_lt_y', y_lt_G'⟩ :
        ∃ y : ℝ, (g' t : EReal) < y ∧ (y : EReal) < G' t :=
      EReal.lt_iff_exists_real_btwn.1
        ((EReal.coe_le_coe_iff.2 (hφg t ht.2)).trans_lt (f_lt_G' t))
    have I1 : ∀ᶠ u in nhdsWithin t (Ioi t),
        (u - t) * y ≤ ∫ w in t..u, (G' w).toReal := by
      have B : ∀ᶠ u in nhds t, (y : EReal) < G' u :=
        G'cont.lowerSemicontinuousAt _ _ y_lt_G'
      rcases mem_nhds_iff_exists_Ioo_subset.1 B with ⟨m, M, ⟨hm, hM⟩, H⟩
      have hnear : Ioo t (min M b) ∈ nhdsWithin t (Ioi t) :=
        Ioo_mem_nhdsGT (lt_min hM ht.right.right)
      filter_upwards [hnear] with u hu
      have Isub : Icc t u ⊆ Icc a b :=
        Icc_subset_Icc ht.2.1 (hu.2.le.trans (min_le_right _ _))
      calc
        (u - t) * y = ∫ _ in Icc t u, y := by
          simp only [MeasureTheory.integral_const, MeasurableSet.univ,
            measureReal_restrict_apply, univ_inter, hu.left.le,
            Real.volume_real_Icc_of_le, smul_eq_mul]
        _ ≤ ∫ w in t..u, (G' w).toReal := by
          rw [intervalIntegral.integral_of_le hu.1.le,
            ← MeasureTheory.integral_Icc_eq_integral_Ioc]
          apply setIntegral_mono_ae_restrict
          · simp
          · exact IntegrableOn.mono_set G'int Isub
          · have C1 : ∀ᵐ x : ℝ ∂volume.restrict (Icc t u), G' x < ∞ :=
              ae_mono (Measure.restrict_mono Isub le_rfl) G'lt_top
            have C2 : ∀ᵐ x : ℝ ∂volume.restrict (Icc t u), x ∈ Icc t u :=
              ae_restrict_mem measurableSet_Icc
            filter_upwards [C1, C2] with x G'x hx
            apply EReal.coe_le_coe_iff.1
            have hx' : x ∈ Ioo m M := by
              simp only [hm.trans_le hx.left,
                (hx.right.trans_lt hu.right).trans_le (min_le_left M b),
                mem_Ioo, and_self_iff]
            refine (H hx').out.le.trans_eq ?_
            exact (EReal.coe_toReal G'x.ne (f_lt_G' x).ne_bot).symm
    have I2 : ∀ᶠ u in nhdsWithin t (Ioi t), g u - g t ≤ (u - t) * y := by
      have g'_lt_y : g' t < y := EReal.coe_lt_coe_iff.1 g'_lt_y'
      filter_upwards
        [(hderiv t ⟨ht.2.1, ht.2.2⟩).limsup_slope_le'
          (notMem_Ioi.2 le_rfl) g'_lt_y, self_mem_nhdsWithin]
        with u hu t_lt_u
      have hmul := mul_le_mul_of_nonneg_left hu.le (sub_pos.2 t_lt_u.out).le
      rwa [← smul_eq_mul, sub_smul_slope] at hmul
    have I3 : ∀ᶠ u in nhdsWithin t (Ioi t),
        g u - g t ≤ ∫ w in t..u, (G' w).toReal := by
      filter_upwards [I1, I2] with u hu1 hu2 using hu2.trans hu1
    have I4 : ∀ᶠ u in nhdsWithin t (Ioi t), u ∈ Ioc t (min v b) :=
      Ioc_mem_nhdsGT (lt_min t_lt_v ht.2.2)
    rcases (I3.and I4).exists with ⟨x, hx, h'x⟩
    refine ⟨x, ?_, Ioc_subset_Ioc le_rfl (min_le_left _ _) h'x⟩
    calc
      g x - g a = g t - g a + (g x - g t) := by abel
      _ ≤ (∫ w in a..t, (G' w).toReal) + ∫ w in t..x, (G' w).toReal :=
        add_le_add ht.1 hx
      _ = ∫ w in a..x, (G' w).toReal := by
        apply intervalIntegral.integral_add_adjacent_intervals
        · rw [intervalIntegrable_iff_integrableOn_Ioc_of_le ht.2.1]
          exact IntegrableOn.mono_set G'int
            (Ioc_subset_Icc_self.trans (Icc_subset_Icc le_rfl ht.2.2.le))
        · rw [intervalIntegrable_iff_integrableOn_Ioc_of_le h'x.1.le]
          apply IntegrableOn.mono_set G'int
          exact Ioc_subset_Icc_self.trans
            (Icc_subset_Icc ht.2.1 (h'x.2.trans (min_le_right _ _)))
  calc
    g b - g a ≤ ∫ y in a..b, (G' y).toReal := main (right_mem_Icc.2 hab)
    _ ≤ (∫ y in a..b, φ y) + ε := by
      convert! hG'.le <;>
        · rw [intervalIntegral.integral_of_le hab]
          simp only [MeasureTheory.integral_Icc_eq_integral_Ioc',
            Real.volume_singleton]

/-- Closed-interval version, obtained from the preceding propagation lemma by
closing the missing endpoint. -/
theorem sub_le_integral_of_hasDeriv_right_of_le
    (hab : a ≤ b)
    (hcont : ContinuousOn g (Icc a b))
    (hderiv : ∀ x ∈ Ioo a b, HasDerivWithinAt g (g' x) (Ioi x) x)
    (φint : IntegrableOn φ (Icc a b))
    (hφg : ∀ x ∈ Ioo a b, g' x ≤ φ x) :
    g b - g a ≤ ∫ y in a..b, φ y := by
  obtain rfl | a_lt_b := hab.eq_or_lt
  · simp
  set s := {t | g b - g t ≤ ∫ u in t..b, φ u} ∩ Icc a b
  have s_closed : IsClosed s := by
    have hc : ContinuousOn (fun t => (g b - g t, ∫ u in t..b, φ u))
        (Icc a b) := by
      rw [← uIcc_of_le hab] at hcont φint ⊢
      exact (continuousOn_const.sub hcont).prodMk
        (intervalIntegral.continuousOn_primitive_interval_left φint)
    simp only [s, inter_comm]
    exact hc.preimage_isClosed_of_isClosed isClosed_Icc isClosed_le_prod
  have A : closure (Ioc a b) ⊆ s := by
    apply s_closed.closure_subset_iff.2
    intro t ht
    refine ⟨?_, ⟨ht.1.le, ht.2⟩⟩
    exact sub_le_integral_of_hasDeriv_right_of_le_Ico ht.2
      (hcont.mono (Icc_subset_Icc ht.1.le le_rfl))
      (fun x hx => hderiv x ⟨ht.1.trans_le hx.1, hx.2⟩)
      (φint.mono_set (Icc_subset_Icc ht.1.le le_rfl))
      (fun x hx => hφg x ⟨ht.1.trans_le hx.1, hx.2⟩)
  rw [closure_Ioc a_lt_b.ne] at A
  exact (A (left_mem_Icc.2 hab)).1

/-- The reverse inequality follows by applying the upper-bound theorem to `-g`. -/
theorem integral_le_sub_of_hasDeriv_right_of_le
    (hab : a ≤ b)
    (hcont : ContinuousOn g (Icc a b))
    (hderiv : ∀ x ∈ Ioo a b, HasDerivWithinAt g (g' x) (Ioi x) x)
    (φint : IntegrableOn φ (Icc a b))
    (hφg : ∀ x ∈ Ioo a b, φ x ≤ g' x) :
    (∫ y in a..b, φ y) ≤ g b - g a := by
  rw [← neg_le_neg_iff]
  convert! sub_le_integral_of_hasDeriv_right_of_le hab hcont.fun_neg
    (fun x hx => (hderiv x hx).neg) φint.neg
    (fun x hx => neg_le_neg (hφg x hx)) using 1
  · abel
  · simp only [← intervalIntegral.integral_neg]
    rfl

/-- Fundamental Theorem of Calculus (FTC-2), for a real-valued function.

Both derivative and integral in the public statement are the definitions made in
this file.  The proof does not call any packaged FTC theorem. -/
theorem fundamental_theorem_of_calculus
    (hab : a ≤ b)
    (hcont : IsContinuousOn g (Icc a b))
    (hderiv : ∀ x ∈ Ioo a b, HasDerivativeAt g (g' x) x)
    (g'int : IsIntegrableOn g' a b) :
    Integral g' a b = g b - g a := by
  have hcont' : ContinuousOn g (Icc a b) := isContinuousOn_iff.mp hcont
  have g'int' : IntegrableOn g' (Icc a b) := isIntegrableOn_iff.mp g'int
  rw [integral_eq_intervalIntegral]
  apply le_antisymm
  · apply integral_le_sub_of_hasDeriv_right_of_le hab hcont'
      (fun x hx => ((hasDerivativeAt_iff_hasDerivAt.mp (hderiv x hx)).hasDerivWithinAt))
      g'int'
    intro _ _
    exact le_rfl
  · apply sub_le_integral_of_hasDeriv_right_of_le hab hcont'
      (fun x hx => ((hasDerivativeAt_iff_hasDerivAt.mp (hderiv x hx)).hasDerivWithinAt))
      g'int'
    intro _ _
    exact le_rfl

end FTC2
end SieveDemo.Calculus
