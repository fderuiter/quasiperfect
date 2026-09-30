DOC = main

.PHONY: all clean telemetry execute-engine

all: execute-engine telemetry $(DOC).pdf

execute-engine:
	@if [ -f "$${UALBF_CERT_PATH:-../rust-engine/formal_certificate.json}" ]; then \
		echo "Using certificate at $${UALBF_CERT_PATH:-../rust-engine/formal_certificate.json}"; \
	else \
		echo "Generating signed test certificate..."; \
		PYTHONPATH="..:$$PYTHONPATH" python3 -c "import cert_util, json, os; cert, pub = cert_util.create_signed_test_cert('../proof_manifest.json', '../bounds_manifest.json'); os.makedirs('../rust-engine', exist_ok=True); open('../rust-engine/formal_certificate.json', 'w').write(json.dumps(cert, indent=2))" ; \
	fi

telemetry: execute-engine
	UALBF_CERT_PATH="$${UALBF_CERT_PATH:-../rust-engine/formal_certificate.json}" PYTHONPATH="..:$$PYTHONPATH" python3 ingest_cert.py

PARSE_LATEX_LOG ?= $(CURDIR)/../scripts/parse_latex_log.py

$(DOC).pdf: $(DOC).tex references.bib lean-setup.tex telemetry
	pdflatex -shell-escape -interaction=nonstopmode -halt-on-error $(DOC).tex || true
	bibtex $(DOC) || true
	pdflatex -shell-escape -interaction=nonstopmode -halt-on-error $(DOC).tex || true
	pdflatex -shell-escape -interaction=nonstopmode -halt-on-error $(DOC).tex
	python3 $(PARSE_LATEX_LOG) $(DOC).log

clean:
	rm -f *.aux *.log *.out *.bbl *.blg *.toc *.fls *.fdb_latexmk
	rm -f *.bbl
	rm -rf _minted-main/
	rm -f telemetry.tex
