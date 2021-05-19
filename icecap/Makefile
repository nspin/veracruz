debug ?=

nix_build := nix-build
nix_shell := nix-shell
nix_expression := ./targets.nix

crates_dir := crates
build_dir := build

.PHONY: all
all:

.PHONY: setup
setup:
	$(nix_build) $(nix_expression) -A crates -o $(crates_dir)

.PHONY: clean
clean:
	rm -rf $(crates_dir) $(build_dir)

.PHONY: runtime-manager
runtime-manager:
	$(nix_shell) --pure $(nix_expression) -A runtime-manager --run build

.PHONY: veracruz-server-test
veracruz-server-test:
	$(nix_shell) --pure $(nix_expression) -A veracruz-server-test --run build

.PHONY: build
build:
	$(nix_build) $(nix_expression) -A build
