nix_build := nix-build -j $(shell nproc)
nix_shell := nix-shell -j $(shell nproc) --pure
nix_expression := ./targets.nix

crates_dir := crates
build_dir := build

.PHONY: all
all:

.PHONY: clean
clean:
	rm -rf $(crates_dir) $(build_dir)

.PHONY: setup
setup:
	$(nix_build) $(nix_expression) -A crates -o $(crates_dir)

.PHONY: runtime-manager
runtime-manager: setup
	$(nix_shell) $(nix_expression) -A runtime-manager --run build

.PHONY: veracruz-server-test
veracruz-server-test: setup
	$(nix_shell) $(nix_expression) -A veracruz-server-test --run build

.PHONY: build
build: runtime-manager veracruz-server-test
	$(nix_build) $(nix_expression) -A build

# HACK
.PHONY: before
before:
	$(MAKE) -C .. test_cases
	cd ../veracruz-server-test && sh populate-test-database.sh


# NOTES
# rustup target add x86_64-unknown-linux-musl
# make sgx
# make nitro