
CARGO_CRATE_ARGS = 	-p manager_api \
					-p manager_model \
					-p app-manager

fmt:
	cargo +nightly fmt $(CARGO_CRATE_ARGS)
fix:
	cargo fix ${CARGO_CRATE_ARGS}

update-api-bindings:
	openapi-generator-cli generate \
	-i http://localhost:4000/api-doc/app_api.json \
	-g rust \
	-o crates/manager_api_client \
	--package-name manager_api_client

code-stats:
	@/bin/echo -n "Lines:"
	@find \
	crates/manager_api \
	crates/manager_model \
	crates/app-manager \
	-name '*.rs' | xargs wc -l | tail -n 1
	@echo "\nCommits:   `git rev-list --count HEAD` total"
