export PYTHONUNBUFFERED=1

# UALBF Project — Monorepo Build Script
#
# Usage:
#   make          — Build Lean proofs, then Rust engine
#   make lean     — Build only Lean (requires mathlib)
#   make rust     — Build only Rust (requires prior `make lean`)
#   make clean    — Clean both Lean and Rust build artifacts
#   make check    — Runs check-core, check-python, and check-paper targets
#   make check-core — Runs Rust formatting and Clippy, and Lean formatting
#   make check-python — Runs Python style checks, type-checking, doc validation, and unit tests
#   make check-paper — Verifies paper unit tests and LaTeX macro synchronization
#   make verify-sync — Verify generated specifications alignment and workspace cleanliness
#
# IMPORTANT: Lean must be built BEFORE Rust, as the Rust engine
# links against the Lean C-IR static library produced by `lake build`.

.PHONY: all lean rust clean audit verification-lib check verify-sync check-core check-python check-paper

all: verification-lib lean rust

audit:
	python3 auditor.py

check: check-core check-python check-paper

check-core:
	cargo fmt --check
	cargo clippy --workspace -- -D warnings
	cargo llvm-cov --workspace --fail-under-functions 80 --summary-only --cobertura --output-path coverage.xml --ignore-filename-regex "(generated|verification_cli|distributed|main\.rs)"
	./scripts/check_lean_fmt.sh
	./scripts/check-code-quality.sh

check-python: verification-lib
	python3 scripts/check_literals.py
	python3 verify_metadata.py
	black --check .
	flake8 .
	mypy .
	python3 scripts/validate_docs.py --check-specs
	python3 scripts/validate_tuning_guide.py
	python3 -m pytest tests paper

check-paper: verification-lib
	python3 scripts/verify_paper_sync.py

verify-sync: verification-lib
	./scripts/export_lean_specs.py
	python3 auditor.py
	@if [ -n "$$(git status --porcelain --untracked-files=no)" ]; then \
		echo "Error: Workspace is dirty after regenerating specifications!"; \
		echo "Out of sync files:"; \
		git status --porcelain --untracked-files=no; \
		echo "To resolve, run:"; \
		echo "  make verify-sync && git add . && git commit -m \"chore: sync generated specifications\""; \
		exit 1; \
	fi


verification-lib:
	cd verification-lib && cargo build --release --features python,signing
	@cp target/release/libverification_lib.so ./verification_lib.so 2>/dev/null || \
	 cp ../target/release/libverification_lib.so ./verification_lib.so 2>/dev/null || \
	 cp verification-lib/target/release/libverification_lib.so ./verification_lib.so 2>/dev/null || \
	 cp target/release/libverification_lib.dylib ./verification_lib.so 2>/dev/null || \
	 cp ../target/release/libverification_lib.dylib ./verification_lib.so 2>/dev/null || \
	 cp verification-lib/target/release/libverification_lib.dylib ./verification_lib.so 2>/dev/null || true

lean: verification-lib
	./scripts/export_lean_specs.py
	cd lean4-proofs && CPATH="../verification-lib/include:../target/include:$$CPATH" lake build UALBF validator

rust: lean
	./scripts/export_lean_specs.py
	python3 auditor.py
	echo "Building WITH signing capabilities (Verified Build)"; \
	cd rust-engine && cargo build --release --features signing;


clean:
	cd lean4-proofs && lake clean
	cd rust-engine && cargo clean
