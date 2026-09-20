# Sieve proof demo

This directory contains two substantial, checked Lean developments intended for
Sieve's proof-graph UI:

- `SieveDemo.Sylow` defines a Sylow subgroup as a maximal `p`-subgroup and proves
  existence (Zorn), conjugacy (the fixed-point action), congruence modulo `p`,
  equality with the normalizer index, and divisibility of the subgroup index.
- `SieveDemo.FTC` defines limits, derivatives as limits of difference quotients,
  and oriented interval integrals.  It proves the real-valued FTC-2 by the
  Vitali--Carathéodory/real-induction argument, including both inequalities.

Neither module imports `Mathlib.GroupTheory.Sylow` nor
`Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus`, and neither
headline theorem forwards to a packaged theorem.  The foundational boundary is
Mathlib's definitions and supporting theory for groups, finite actions, real
topology, and the Bochner integral (whose construction from simple functions is
reused); all Sylow and FTC-specific arguments are local to this directory.

Build the demo:

```sh
lake build SieveDemo
```

Inspect the main proof graphs:

```sh
cargo run -- --import SieveDemo.Sylow proof-steps \
  SieveDemo.card_sylow_modEq_one

cargo run -- --import SieveDemo.Sylow proof-steps \
  SieveDemo.Sylow.card_eq_index_normalizer

cargo run -- --import SieveDemo.FTC proof-steps \
  SieveDemo.Calculus.fundamental_theorem_of_calculus
```

Open both modules together in the graph UI:

```sh
cargo run -- --import SieveDemo.Sylow --import SieveDemo.FTC serve
```
