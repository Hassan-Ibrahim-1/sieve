import Lake

open Lake DSL

package sieve

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @ "master"

lean_lib Sieve

@[default_target]
lean_exe sieve_extract where
  root := `Sieve.Extract
