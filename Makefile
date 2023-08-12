
CARGO_CRATE_ARGS = 	-p manager_api \
					-p manager_model \
					-p app-manager

fmt:
	cargo fmt $(CARGO_CRATE_ARGS)
fix:
	cargo fix ${CARGO_CRATE_ARGS}
