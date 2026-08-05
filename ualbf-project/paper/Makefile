DOC = main

.PHONY: all clean telemetry

all: telemetry $(DOC).pdf

telemetry:
	PYTHONPATH="..:$$PYTHONPATH" python3 ingest_cert.py

$(DOC).pdf: $(DOC).tex references.bib lean-setup.tex telemetry
	pdflatex -shell-escape -interaction=nonstopmode -halt-on-error $(DOC).tex || true
	bibtex $(DOC) || true
	pdflatex -shell-escape -interaction=nonstopmode -halt-on-error $(DOC).tex || true
	pdflatex -shell-escape -interaction=nonstopmode -halt-on-error $(DOC).tex
	python3 ../scripts/parse_latex_log.py $(DOC).log

clean:
	latexmk -c
	rm -f *.bbl
	rm -rf _minted-main/
	rm -f telemetry.tex
