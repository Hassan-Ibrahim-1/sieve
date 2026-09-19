# FTC Slop

This is a deliberately small, self-contained Lean project for exercising
Sieve. It imports Lake only; it does not depend on Mathlib.

The development models calculus on `Nat → Int`:

- `derivative f n` is the forward difference `f (n + 1) - f n`.
- `integral f a n` is the finite sum of those differences over `n` steps.
- `fundamental_theorem_of_calculus` is proved by induction and telescoping.

It is therefore a checked discrete FTC analogue, not a real-analysis
formalization with limits, continuity, or Riemann/Lebesgue integration.

```sh
lake build
lake env lean --run FtcSlop/Main.lean
./.lake/build/bin/ftc_slop
```

Both executable commands print exactly:

```text
FtcSlop: integral square 0 4 = 16
```

`FtcSlop.Calculus` contains the declarations to analyze. `FtcSlop` is only an
import wrapper, so Sieve intentionally reports an empty direct-definition corpus
for that module. From the repository root, run:

```sh
cargo run -- --project examples/ftc-slop \
  --import FtcSlop.Calculus summary

cargo run -- --project examples/ftc-slop \
  --import FtcSlop.Calculus discover --lens all

cargo run -- --project examples/ftc-slop \
  --import FtcSlop.Calculus proof-steps \
  FtcSlop.fundamental_theorem_of_calculus
```

The default summary contains seven authored declarations (two definitions and
five theorems). Add `--include-generated` to include the two equation theorems,
for nine declarations and seven theorems total.
