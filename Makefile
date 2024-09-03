.PHONY: install

install:
	Rscript -e "rextendr::document(); devtools::install()"
