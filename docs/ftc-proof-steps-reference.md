# FTC proof-step extraction reference

The milestone reference is
`intervalIntegral.integral_eq_sub_of_hasDeriv_right_of_le` from Mathlib's
interval-integral fundamental theorem of calculus module. Its proof transports
the scalar FTC to a complete real normed vector space.

The mathematical outline used to evaluate extraction is:

1. Reduce the vector equality
   `∫ y in a..b, f' y = f b - f a` to equality after applying every continuous
   linear functional, using `SeparatingDual.eq_iff_forall_dual_eq`.
2. For an arbitrary functional `g`, commute `g` through the interval integral
   and through subtraction, using
   `ContinuousLinearMap.intervalIntegral_comp_comm` and
   `ContinuousLinearMap.map_sub`.
3. Apply the real-valued theorem
   `intervalIntegral.integral_eq_sub_of_hasDeriv_right_of_le_real` to `g ∘ f`.
   Its continuity, derivative, and integrability premises come respectively
   from composition with `g`, the chain rule, and integrability under a
   continuous linear map.

This document is an evaluation oracle, not an extraction rule. The extractor
must find these claims from the elaborated proof term and Lean's inferred types;
it must not special-case any of the declarations above.

The raw report succeeds when a reader can locate all three stages, see the
local assumptions (including the arbitrary functional), and follow
prerequisite-to-claim edges to the theorem conclusion. Elaborator
infrastructure may remain visible in the raw graph. Removing that detail is a
later outline-condensation task, and every condensed edge will need provenance
back to paths in this graph.
