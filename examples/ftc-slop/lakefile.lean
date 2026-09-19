import Lake

open Lake DSL

package ftc_slop

lean_lib FtcSlop

@[default_target]
lean_exe ftc_slop where
  root := `FtcSlop.Main
