import Lake
open System Lake DSL

package ualbf where
  moreLinkArgs := #["-L../target/release", "-L../target/debug", "-L../verification-lib/target/release", "-lverification_lib"]
  -- Conditionally treat compiler warnings as fatal errors only when requested,
  -- ensuring third-party community dependencies are not broken by warning-as-error.
  moreLeanArgs := if (get_config? warnings_as_errors).isSome then #["-DwarningAsError=true"] else #[]

require mathlib from git "https://github.com/leanprover-community/mathlib4.git" @ "v4.30.0"

input_file ffi.c where
  path := "ffi.c"
  text := true

input_file verification_lib.h where
  path := "include/verification_lib.h"
  text := true

target ffi.o pkg : FilePath := do
  let oFile := pkg.buildDir / "c" / "ffi.o"
  let srcJob ← ffi.c.fetch
  let headerJob ← verification_lib.h.fetch
  let srcJob := (srcJob.mix headerJob).map fun _ => pkg.dir / "ffi.c"
  let flags := #["-I", (← getLeanIncludeDir).toString, "-I", "include", "-I", (pkg.dir / "include").toString, "-fPIC"]
  buildO oFile srcJob flags #[] "cc"

target libleanffi pkg : FilePath := do
  let name := nameToStaticLib "leanffi"
  let ffiO ← ffi.o.fetch
  buildStaticLib (pkg.staticLibDir / name) #[ffiO]

lean_lib UALBF where
  moreLinkObjs := #[libleanffi]

lean_exe validator where
  root := `Validator
  moreLinkObjs := #[libleanffi]
