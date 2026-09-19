import Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus

open MeasureTheory Set

namespace Sieve.SlopCorpus

variable {E : Type*} [NormedAddCommGroup E] [NormedSpace ℝ E] [CompleteSpace E]
variable {f : ℝ → E} {a b : ℝ}

/-- Deliberately noisy: an irrelevant proposition and avoidable local rewrites
surround an otherwise ordinary calculus statement. -/
theorem resultOne (unused : True) (hderiv : ∀ x ∈ Set.uIcc a b, DifferentiableAt ℝ f x)
    (hint : IntervalIntegrable (deriv f) volume a b) :
    ∫ y in a..b, deriv f y = f b - f a := by
  have stillUnused : True := unused
  have mainResult := intervalIntegral.integral_deriv_eq_sub hderiv hint
  have copiedResult : ∫ y in a..b, deriv f y = f b - f a := mainResult
  clear stillUnused mainResult
  exact copiedResult

/-- Nearly the same noisy statement, with a vacuous endpoint equality instead
of `True` and a round trip through symmetry. -/
theorem resultTwo (unused : a = a) (hderiv : ∀ x ∈ Set.uIcc a b, DifferentiableAt ℝ f x)
    (hint : IntervalIntegrable (deriv f) volume a b) :
    ∫ y in a..b, deriv f y = f b - f a := by
  have endpointNoise : a = a := unused
  have mainResult := intervalIntegral.integral_deriv_eq_sub hderiv hint
  have backwards : f b - f a = ∫ y in a..b, deriv f y := mainResult.symm
  clear endpointNoise mainResult
  exact backwards.symm

/-- A third close variant: the irrelevant fact mentions the other endpoint and
the proof inserts an unnecessary equality transport. -/
theorem resultThree (unused : b = b) (hderiv : ∀ x ∈ Set.uIcc a b, DifferentiableAt ℝ f x)
    (hint : IntervalIntegrable (deriv f) volume a b) :
    ∫ y in a..b, deriv f y = f b - f a := by
  have endpointNoise : b = b := unused
  let claimed : Prop := ∫ y in a..b, deriv f y = f b - f a
  have mainResult : claimed := by
    change ∫ y in a..b, deriv f y = f b - f a
    exact intervalIntegral.integral_deriv_eq_sub hderiv hint
  clear endpointNoise
  change claimed
  exact mainResult

end Sieve.SlopCorpus
