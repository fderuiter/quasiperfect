{
  description = "UALBF Nix-Based Reproducibility Suite";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          config = {
            allowDeprecatedx86_64Darwin = "force";
          };
        };

        verusPkg = let
          systemMap = {
            "x86_64-linux" = {
              name = "verus-0.2026.07.18.3a4d30b-x86-linux.zip";
              sha256 = "7097a91ea4bf5896a418d90743626cbe5c085ce5ef8a64ed8d84c0aa5e49ac55";
            };
            "x86_64-darwin" = {
              name = "verus-0.2026.07.18.3a4d30b-x86-macos.zip";
              sha256 = "7eebd809babe59ef72fd619411c3b89f8becd2d6500bf01abad88d0595828f35";
            };
            "aarch64-darwin" = {
              name = "verus-0.2026.07.18.3a4d30b-arm64-macos.zip";
              sha256 = "74958a303e97ea7a267caead7ff8a51321af83855be6e33e6d4135a5e9c9b475";
            };
          };
          systemInfo = systemMap.${system} or systemMap."x86_64-linux";
        in pkgs.stdenv.mkDerivation {
          pname = "verus";
          version = "0.2026.07.18.3a4d30b";
          src = pkgs.fetchurl {
            url = "https://github.com/verus-lang/verus/releases/download/release/0.2026.07.18.3a4d30b/${systemInfo.name}";
            sha256 = "${systemInfo.sha256}";
          };
          nativeBuildInputs = [ pkgs.unzip ] ++ pkgs.lib.optional pkgs.stdenv.isLinux pkgs.autoPatchelfHook;
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.stdenv.cc.cc.lib
            pkgs.zlib
            pkgs.gmp
            rustToolchain
          ];
          unpackCmd = "unzip $curSrc";
          installPhase = ''
            mkdir -p $out/bin
            VERUS_DIR=$(dirname $(find . -type f -name verus | head -n 1))
            cp -r $VERUS_DIR/* $out/bin/
            chmod +x $out/bin/verus $out/bin/rust_verify $out/bin/z3 || true
          '';
        };

        rustToolchain = let
          systemMap = {
            "x86_64-linux" = {
              url = "https://static.rust-lang.org/dist/rust-1.96.0-x86_64-unknown-linux-gnu.tar.xz";
              sha256 = "c295047583a56238ea06b43f849f4b877fa12bfd4c7103f8d9a74c94c9c4e108";
            };
            "x86_64-darwin" = {
              url = "https://static.rust-lang.org/dist/rust-1.96.0-x86_64-apple-darwin.tar.xz";
              sha256 = "63a6d717a5e5392ac43f0a1593e7aabe6128c8685d318cb890603b1688cb3339";
            };
            "aarch64-darwin" = {
              url = "https://static.rust-lang.org/dist/rust-1.96.0-aarch64-apple-darwin.tar.xz";
              sha256 = "f04a974f3579d3524f6b9bc6490a27c9fb358050e7cd8a641945f30bf24c1dce";
            };
          };
          systemInfo = systemMap.${system} or systemMap."x86_64-linux";
        in pkgs.stdenv.mkDerivation {
          pname = "rust-toolchain";
          version = "1.96.0";
          src = pkgs.fetchurl {
            url = systemInfo.url;
            sha256 = systemInfo.sha256;
          };
          nativeBuildInputs = pkgs.lib.optional pkgs.stdenv.isLinux pkgs.autoPatchelfHook;
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.zlib
            pkgs.curl
            pkgs.openssl
            pkgs.stdenv.cc.cc.lib
          ];
          dontStrip = true;
          installPhase = ''
            patchShebangs .
            ./install.sh --prefix=$out --disable-ldconfig
          '';
        };

        lakeManifest = builtins.fromJSON (builtins.readFile ./ualbf-project/lean4-proofs/lake-manifest.json);
        
        linkPackages = pkgs.lib.concatStringsSep "\n" (map (pkg: ''
          mkdir -p .lake/packages/${pkg.name}
          cp -rT ${builtins.fetchGit { url = pkg.url; rev = pkg.rev; submodules = true; }} .lake/packages/${pkg.name}
          chmod -R +w .lake/packages/${pkg.name}
        '') lakeManifest.packages);

        rewriteManifest = ''
          jq '
            .packages |= map(
              if .type == "git" then
                .type = "path" | .dir = ".lake/packages/" + .name
              else . end
            )
          ' lake-manifest.json > lake-manifest.json.tmp
          mv lake-manifest.json.tmp lake-manifest.json
          sed -i 's/from git ".*"/from ".lake\/packages\/mathlib"/g' lakefile.lean
        '';

        leanDeps = pkgs.stdenv.mkDerivation {
          pname = "lean-deps";
          version = "0.1.0";
          src = pkgs.lib.cleanSourceWith {
            src = ./ualbf-project/lean4-proofs;
            filter = path: type: builtins.match ".*(lake-manifest.json|lakefile.lean|lean-toolchain|lakefile.toml)$" path != null || type == "directory";
          };
          nativeBuildInputs = [ pkgs.lean4 pkgs.git pkgs.cacert pkgs.jq pkgs.curl ];
          buildPhase = ''
            export HOME=$TMPDIR
            export GIT_SSL_CAINFO="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            ${linkPackages}
            ${rewriteManifest}
            if grep -q "mathlib" lake-manifest.json; then
              lake exe cache get || true
            fi
            # We specifically build proofwidgets to fetch the JS assets
            lake build proofwidgets || true
            lake build LeanSearchClient || true
            # Clean up locally compiled files that contain the FOD store path to maintain hash reproducibility
            echo "Cleaning up compiled files to prevent store path leaks..."
            # Delete all compiled files except those in mathlib (which are from cache and safe)
            find .lake -type f \( -name '*.olean' -o -name '*.ilean' -o -name '*.c' -o -name '*.o' \) | grep -v "\.lake/packages/mathlib" | xargs rm -f || true
            find .lake -type f -name '*.trace' -delete || true
            find .lake -type f -name '*.hash' -delete || true
            find .lake -name 'lake-manifest.json.tmp' -delete || true


            # Clean up compiled files to avoid hash non-determinism?
            # Actually, `lake exe cache get` creates .olean files. Let's keep them so the main build is fast.

          '';
          installPhase = ''
            # Remove any binaries built which might contain nix store paths
            find .lake -type f -name cache.rsp -delete || true
            find .lake -type f -name cache -delete || true
            rm -rf .lake/packages/mathlib/.lake/build/bin || true
            mkdir -p $out
            cp -r .lake $out/
          '';
          dontFixup = true;
          outputHashAlgo = "sha256";
          outputHashMode = "recursive";
          outputHash = "sha256-JzoxPKsQ9uNNlHZo9dbhpo63MWjfOoCWbYLhVZV1LCk=";
        };

        leanPkg = pkgs.stdenv.mkDerivation {
          pname = "ualbf-lean4-proofs";
          version = "0.1.0";
          src = ./ualbf-project/lean4-proofs;

          nativeBuildInputs = [ pkgs.lean4 pkgs.git pkgs.cacert pkgs.jq ];

          preBuild = ''
            chmod +w ..
            mkdir -p ../verification-lib/target/release
            ln -s ${verificationLib}/lib/libverification_lib.* ../verification-lib/target/release/ || true
          '';

          buildPhase = ''
            export HOME=$TMPDIR
            export GIT_SSL_CAINFO="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
            cp -r ${leanDeps}/.lake .lake
            chmod -R +w .lake
            ${rewriteManifest}
            lake build
          '';

          installPhase = ''
            mkdir -p $out
            cp -r .lake $out/
          '';
        };


        verificationLib = pkgs.rustPlatform.buildRustPackage {
          pname = "verification-lib";
          version = "0.1.0";
          src = pkgs.lib.cleanSourceWith {
            src = ./ualbf-project;
            filter = path: type:
              let 
                p = toString path;
              in
                builtins.match ".*(Cargo\\.toml|Cargo\\.lock|verification-lib.*|rust-engine.*)$" p != null || type == "directory";
          };
          buildAndTestSubdir = "verification-lib";

          cargoBuildFlags = [ "-p" "verification-lib" ];
          cargoTestFlags = [ "-p" "verification-lib" ];

          cargoLock = {
            lockFile = ./ualbf-project/Cargo.lock;
          };

          nativeBuildInputs = [
            pkgs.python3
            pkgs.pkg-config
            pkgs.llvmPackages.libclang
          ];

          buildInputs = [
            pkgs.z3
            pkgs.libcxx
          ] ++ pkgs.lib.optionals (!pkgs.stdenv.isDarwin) [
            pkgs.pkgsStatic.gmp
            pkgs.pkgsStatic.libuv
            pkgs.ocl-icd
            pkgs.opencl-headers
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.gmp
            pkgs.libuv
          ];

          buildFeatures = [ "python" ];
          
          preBuild = ''
            chmod +w ..
            export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
          '';

          installPhase = ''
            mkdir -p $out/lib
            find target -name "libverification_lib.*" -exec cp {} $out/lib/ \; || true
            find ../target -name "libverification_lib.*" -exec cp {} $out/lib/ \; || true
          '';
        };

        ualbfEngine = pkgs.rustPlatform.buildRustPackage {
          pname = "ualbf-engine";
          version = "0.1.0";
          src = pkgs.lib.cleanSourceWith {
            src = ./ualbf-project;
            filter = path: type:
              let 
                p = toString path;
              in
                builtins.match ".*(Cargo\\.toml|Cargo\\.lock|rust-engine.*|verification-lib.*|scripts.*|bounds_manifest\\.json|lean4-proofs.*)$" p != null || type == "directory";
          };

          sourceRoot = "source/rust-engine";

          cargoLock = {
            lockFile = ./ualbf-project/Cargo.lock;
          };

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.lean4
            pkgs.llvmPackages.libclang
            pkgs.m4
          ];

          buildInputs = [
            pkgs.z3
            pkgs.libcxx
          ] ++ pkgs.lib.optionals (!pkgs.stdenv.isDarwin) [
            pkgs.pkgsStatic.gmp
            pkgs.pkgsStatic.libuv
            pkgs.ocl-icd
            pkgs.opencl-headers
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.gmp
            pkgs.libuv
          ];

          # Symlink the built Lean objects so build.rs can find them.
          preBuild = ''
            chmod +w ..
            mkdir -p ../lean4-proofs
            chmod -R +w ../lean4-proofs || true
            ln -s ${leanPkg}/.lake ../lean4-proofs/.lake
            export LEAN_SYSROOT="${pkgs.lean4}"
            export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
            # Explicitly export Z3_SYS_Z3_HEADER so z3-sys instantly finds the precompiled system Z3 headers
            # from the Nix store rather than compiling Z3 from source.
            # We use pkgs.z3.dev because Nixpkgs splits z3 into multiple outputs (out and dev),
            # and z3.h is packaged only under the dev output path.
            # This avoids compilation failures during the Build and Verify phase.
            export Z3_SYS_Z3_HEADER="${pkgs.z3.dev}/include/z3.h"
          '';
        };

      in
      {
        packages = {
          default = ualbfEngine;
          engine = ualbfEngine;
          leanDeps = leanDeps;
          lean = leanPkg;
          verus = verusPkg;
        };

        checks = {
          rust-literals = pkgs.stdenv.mkDerivation {
            pname = "rust-literals-check";
            version = "0.1.0";
            src = ./ualbf-project;

            nativeBuildInputs = [ pkgs.python3 ];

            buildPhase = ''
              echo "Running Rust literal validation..."
              python3 scripts/check_literals.py
            '';

            installPhase = ''
              mkdir -p $out
              touch $out/success
            '';
          };

          latex-paper = pkgs.stdenv.mkDerivation {
            pname = "latex-paper-check";
            version = "0.1.0";
            src = ./ualbf-project;

            nativeBuildInputs = [ 
              pkgs.python3 
              pkgs.python3Packages.pygments 
              (if pkgs ? texliveFull then pkgs.texliveFull else pkgs.texlive.combined.scheme-full)
              pkgs.gnumake 
              pkgs.which
            ];

            buildPhase = ''
              export HOME=$TMPDIR
              echo "Setting up verification-lib..."
              cp ${verificationLib}/lib/libverification_lib.so ./verification_lib.so || cp ${verificationLib}/lib/libverification_lib.dylib ./verification_lib.so || cp ${verificationLib}/lib/libverification_lib.* ./verification_lib.so
              
              echo "Patching argparse for latexminted..."
              cp $(python3 -c "import argparse; print(argparse.__file__)") paper/argparse.py
              sed -i 's/parser = self._parser_class(\*\*kwargs)/kwargs.pop("color", None); parser = self._parser_class(\*\*kwargs)/g' paper/argparse.py
              export PYTHONPATH=$PWD:$PWD/paper:$PYTHONPATH

              echo "Generating dummy certificate..."
              python3 -c '
import json, hashlib
with open("proof_manifest.json", "rb") as f:
    manifest_hash = hashlib.sha256(f.read()).hexdigest()
with open("bounds_manifest.json", "r") as f:
    bounds = json.load(f)
cert = {
    "manifest_hash": manifest_hash,
    "verified_logic_hash": "dummy",
    "telemetry": {
        "phase2_execution_time_ms": 1000,
        "total_branches_searched": 10,
        "abundance_pruned": 0,
        "raycast_pruned": 0,
        "target_min_log10": bounds["search_bounds"]["target_min_log10"]["value"],
        "target_max_log10": bounds["search_bounds"]["target_max_log10"]["value"],
        "phase1_pruned": 0
    },
    "engine_version": "dummy",
    "commit_hash": "dummy"
}
with open("dummy_cert.json", "w") as f:
    json.dump(cert, f)
'
              export UALBF_CERT_PATH=$PWD/dummy_cert.json
              export UALBF_DUMMY_PAPER_CI=1
              export PYTHONPATH=$PWD:$PWD/paper:$PYTHONPATH
              
              echo "Compiling LaTeX paper..."
              cd paper
              make all
              cd ..
            '';

            installPhase = ''
              mkdir -p $out
              cp paper/main.pdf $out/
              touch $out/success
            '';
          };
          formatting = pkgs.stdenv.mkDerivation {
            pname = "lean-formatting-check";
            version = "0.1.0";
            src = ./ualbf-project/lean4-proofs;
            
            nativeBuildInputs = [ pkgs.gawk pkgs.diffutils pkgs.coreutils ];
            
            buildPhase = ''
              echo "Checking for trailing whitespace, tabs, and missing newlines..."
              BAD_FILES=0
              
              while IFS= read -r file; do
                # Process the file to fix trailing spaces, convert tabs to spaces, and ensure a trailing newline
                awk '
                  { 
                    sub(/[ \t]+$/, "");  # Remove trailing whitespace
                    gsub(/\t/, "    ");  # Replace tabs with 4 spaces
                    print
                  }
                ' "$file" > "$file.fmt"
                
                # Check for differences
                if ! diff -u "$file" "$file.fmt"; then
                  echo "Formatting error in $file"
                  BAD_FILES=1
                fi
              done < <(find . -type d -name ".lake" -prune -o -type f -name "*.lean" -print)
              
              if [ $BAD_FILES -ne 0 ]; then
                echo "Formatting checks failed. Please fix the files listed above."
                exit 1
              fi
              
              echo "All files formatted correctly!"
            '';
            
            installPhase = ''
              mkdir -p $out
              touch $out/success
            '';
          };

          warnings = pkgs.stdenv.mkDerivation {
            pname = "ualbf-lean4-warnings-check";
            version = "0.1.0";
            src = ./ualbf-project/lean4-proofs;

            nativeBuildInputs = [ pkgs.lean4 pkgs.git pkgs.cacert pkgs.jq ];

            preBuild = ''
              chmod +w ..
              mkdir -p ../verification-lib/target/release
              ln -s ${verificationLib}/lib/libverification_lib.* ../verification-lib/target/release/ || true
            '';

            buildPhase = ''
              echo "Building Lean project with warnings treated as errors..."
              # Pass warnings_as_errors configuration to treat compiler warnings in ualbf as fatal errors
              export HOME=$TMPDIR
              export GIT_SSL_CAINFO="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
              cp -r ${leanDeps}/.lake .lake
            chmod -R +w .lake
            ${rewriteManifest}
              lake build -Kwarnings_as_errors
            '';

            installPhase = ''
              mkdir -p $out
              touch $out/success
            '';
          };

          verus-proofs = pkgs.stdenv.mkDerivation {
            pname = "verus-proofs-check";
            version = "0.1.0";
            src = ./.;

            nativeBuildInputs = [ verusPkg rustToolchain ];

            VERUS_USE_RUSTUP = "0";
            LD_LIBRARY_PATH = "${rustToolchain}/lib";
            DYLD_LIBRARY_PATH = "${rustToolchain}/lib";

            buildPhase = ''
              echo "Running Verus semantic proof verification..."
              export HOME=$TMPDIR
              cd ualbf-project/rust-engine/src
              verus --crate-type=lib verus_proofs.rs
            '';

            installPhase = ''
              mkdir -p $out
              touch $out/success
            '';
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            pkgs.m4
            rustToolchain
          ];
          buildInputs = [
            pkgs.lean4
            (pkgs.python3.withPackages (ps: with ps; [
              black
              flake8
              mypy
              pytest
              pytest-mock
              cryptography
            ]))
            pkgs.z3
            # Include development headers of z3 so that python test subprocesses compiling z3-sys can locate z3.h
            # This development package is critical for the 'Build and Verify' (check_run_id: 93101214645) gating checks
            # to run successfully in the Nix sandbox.
            pkgs.z3.dev
            pkgs.pkg-config
            pkgs.llvmPackages.libclang
            pkgs.libcxx
          ] ++ pkgs.lib.optionals (!pkgs.stdenv.isDarwin) [
            pkgs.pkgsStatic.gmp
            pkgs.pkgsStatic.libuv
            pkgs.ocl-icd
            pkgs.opencl-headers
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.gmp
            pkgs.libuv
          ];

          LEAN_SYSROOT = "${pkgs.lean4}";
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          # We use pkgs.z3.dev as Nixpkgs separates development headers from standard package outputs
          Z3_SYS_Z3_HEADER = "${pkgs.z3.dev}/include/z3.h";

          shellHook = ''
            export LEAN_SYSROOT="${pkgs.lean4}"
            export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"
            export Z3_SYS_Z3_HEADER="${pkgs.z3.dev}/include/z3.h"
          '';
        };
      }
    );
}
