.PHONY: install

document:
	Rscript -e "rextendr::document()"

install:
	Rscript -e "devtools::install()"
