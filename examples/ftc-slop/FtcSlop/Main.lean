import FtcSlop

namespace FtcSlop

def square (n : Nat) : Int := Int.ofNat (n * n)

theorem square_ftc : integral square 0 4 = square 4 - square 0 := by
  simpa using fundamental_theorem_of_calculus_at_zero square 4

end FtcSlop

def main : IO Unit := do
  let result := FtcSlop.integral FtcSlop.square 0 4
  if result != 16 then
    throw (IO.userError s!"unexpected discrete integral: {result}")
  IO.println s!"FtcSlop: integral square 0 4 = {result}"
