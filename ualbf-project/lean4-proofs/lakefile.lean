import Lake
open System Lake DSL

package ualbf where
  moreLinkArgs := #["-L../verification-lib/target/release", "-lverification_lib"]
  -- Conditionally treat compiler warnings as fatal errors only when requested,
  -- ensuring third-party community dependencies are not broken by warning-as-error.
  moreLeanArgs := if (get_config? warnings_as_errors).isSome then #["-DwarningAsError=true"] else #[]

require mathlib from git "https://github.com/leanprover-community/mathlib4.git" @ "v4.30.0"

input_file ffi.c where
  path := "ffi.c"
  text := true

target ffi.o pkg : FilePath := do
  let oFile := pkg.buildDir / "c" / "ffi.o"
  let srcJob ← ffi.c.fetch
  let flags := #["-I", (← getLeanIncludeDir).toString, "-fPIC"]
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
