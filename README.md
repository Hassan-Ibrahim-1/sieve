# Sieve

Sieve extracts structural information from elaborated Lean declarations so that
formal proofs can be inspected with statistics and visualizations.

The initial test corpus is a three-theorem slice of Mathlib's Fundamental
Theorem of Calculus module:

- `intervalIntegral.integral_deriv_eq_sub`
- `intervalIntegral.integral_deriv_eq_sub'`
- `intervalIntegral.integral_deriv_eq_sub_uIoo`

The Lean extractor loads the compiled Mathlib module, obtains each declaration
from the Lean environment, and emits JSON. The Rust process deserializes the
snapshot and keeps it in memory.

## Run

```sh
lake update
cargo run
```

Pass fully qualified declaration names to inspect a custom subset:

```sh
cargo run -- intervalIntegral.integral_deriv_eq_sub
```

## Test

```sh
lake build sieve_extract
cargo test
```
