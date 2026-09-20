import Lake

open Lake DSL

package sieve

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git" @
    "dec5b2b780537b6eaf7f5e5f000c12f7387fb24d"

lean_lib Sieve

lean_lib MathlibDemo where
  srcDir := "examples/mathlib-demo"

lean_lib SieveDemo where
  srcDir := "demo"

@[default_target]
lean_exe sieve_extract where
  root := `Sieve.Extract
  supportInterpreter := true
