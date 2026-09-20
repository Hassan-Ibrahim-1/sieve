# Mathlib Demo

This example contains local copies of substantial mathlib proof bodies for Sieve demos.
The files are wired into the root Lake project as the `MathlibDemo` library so they can
reuse the repository's pinned mathlib checkout.

Build:

```sh
lake build MathlibDemo.FTC MathlibDemo.Sylow
```

Run Sieve:

```sh
cargo run -- --import MathlibDemo.FTC summary
cargo run -- --import MathlibDemo.Sylow summary
```

Inspect proof steps:

```sh
cargo run -- --import MathlibDemo.FTC proof-steps \
  MathlibDemo.FTC.fundamental_theorem_of_calculus_demo

cargo run -- --import MathlibDemo.Sylow proof-steps \
  MathlibDemo.Sylow.card_sylow_modEq_one_demo
```

Start the graph UI:

```sh
cargo run -- --import MathlibDemo.FTC --import MathlibDemo.Sylow serve
```
