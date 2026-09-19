# Sieve

Sieve extracts structural information from elaborated Lean declarations so that
formal proofs can be inspected with statistics and visualizations.

The initial test corpus is Mathlib's Fundamental Theorem of Calculus module,
pinned to commit `dec5b2b780537b6eaf7f5e5f000c12f7387fb24d`.
It currently contains 104 declarations, including 90 theorems and generated or
private declarations. Three useful focus theorems are:

- `intervalIntegral.integral_deriv_eq_sub`
- `intervalIntegral.integral_deriv_eq_sub'`
- `intervalIntegral.integral_deriv_eq_sub_uIoo`

The Lean extractor loads the compiled Mathlib module, obtains each declaration
from the Lean environment, and emits JSON. The Rust process deserializes the
snapshot, validates it, and keeps it in memory. The full snapshot is about 8.7
MB as uncompressed JSON; no analysis database is used.

## Extracted data

For every corpus declaration, Sieve retains:

- declaration kind, module, source range, documentation, universe parameters,
  and internal/private/unsafe/partial flags;
- the pretty-printed type and transitive axioms;
- complete hash-consed expression DAGs for the type and value/proof;
- expression kinds, child edges, constants, binders, de Bruijn indices,
  projections, literals, and universe levels;
- direct statement and value dependencies, including occurrence counts;
- aggregate expression-shape statistics.

The snapshot also contains a symbol table for every directly referenced Lean
declaration. Symbol metadata includes its defining module, declaration kind,
and whether it is a class, instance, or projection. This allows dependency
statistics to distinguish mathematical declarations from common elaboration
infrastructure.

Rust builds declaration indexes, typed dependency edges, reverse-dependency
indexes, and occurrence multiplicities from this raw data. The stored DAGs can
support repeated-subexpression analysis and structural comparison without
losing the original occurrence counts. Sieve also derives raw and
alpha-normalized structural fingerprints; these identify comparison candidates
while the complete DAGs remain available for collision-safe equality checks.

Tactic invocations and before/after goal states are not part of this compiled
snapshot. They require a separate source re-elaboration pipeline and should not
be inferred from the final proof term.

## Run

```sh
lake update
cargo run
```

Pass fully qualified declaration names to inspect a custom subset:

```sh
cargo run -- intervalIntegral.integral_deriv_eq_sub
```

Inspect a bounded prefix of an elaborated proof tree:

```sh
cargo run -- --tree-depth 4 intervalIntegral.integral_deriv_eq_sub
```

## Test

```sh
lake build sieve_extract
cargo test
```

The full-module integration test is intentionally excluded from the normal
test run because loading and validating the entire compiled Mathlib module is
slow. Run it explicitly with:

```sh
cargo test extracts_an_analysis_ready_ftc_module -- --ignored
```
