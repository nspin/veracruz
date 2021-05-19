nix_build ?= nix-build
nix_shell ?= nix-shell
nix_expression ?= ./.
build_dir ?= build
debug ?=

.PHONY: all
all:

.PHONY: setup
setup:
	$(nix_build) $(nix_expression) -A virt.icecapCrates -o crates

.PHONY: clean
clean:
	rm -rf crates build

.PHONY: runtime-manager
runtime-manager:
	$(nix_shell) --pure $(nix_expression) -A virt.runtime-manager --run build

.PHONY: veracruz-server-test
veracruz-server-test:
	$(nix_shell) --pure $(nix_expression) -A virt.veracruz-server-test --run build

.PHONY: build
build:
	$(nix_build) $(nix_expression) -A virt.run
