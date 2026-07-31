-- UALBF: Universal Arithmetic Lower Bound Framework
-- Root import file for the Lean 4 proof library

-- Layer 0: Definitions
import UALBF.Basic

-- Layer 1: Pure Mathematics (no QPN hypothesis)
import UALBF.Pure.Arithmetic
import UALBF.Pure.RationalBounds
import UALBF.Pure.EulerProduct
import UALBF.Pure.Cyclotomic
import UALBF.Pure.CyclotomicAlgebra
import UALBF.Pure.Zsigmondy
import UALBF.Pure.Fixed64

-- Layer 2: QPN Theory (requires IsQuasiperfect)
import UALBF.QPN.BasicProperties
import UALBF.QPN.Obstruction
import UALBF.QPN.TouchardQPN
import UALBF.QPN.PrasadSunitha
import UALBF.QPN.AbundancyBound

-- Layer 3: Engine Verification
import UALBF.Engine.SearchState
import UALBF.Engine.Bipartition
import UALBF.Engine.SieveSoundness
import UALBF.Engine.Obstruction
import UALBF.Engine.SearchMonad
import UALBF.BloomFilter

-- FFI: C-linkage exports
import UALBF.FFI

import UALBF.Engine.Mod5Bridge
import UALBF.Engine.TouchardBridge
