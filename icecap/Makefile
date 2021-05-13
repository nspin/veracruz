nix_build := nix-build -j $(shell nproc)
nix_shell := nix-shell -j $(shell nproc) --pure
nix_expression := ./targets.nix

crates_dir := crates
build_dir := build
now := $(build_dir)/NOW

.PHONY: all
all:

.PHONY: clean
clean:
	rm -rf $(crates_dir) $(build_dir)

.PHONY: setup
setup:
	$(nix_build) $(nix_expression) -A crates -o $(crates_dir)

.PHONY: runtime-manager
runtime-manager: $(now) setup
	$(nix_shell) $(nix_expression) -A $@ --run build

.PHONY: veracruz-server-test
veracruz-server-test: setup
	$(nix_shell) $(nix_expression) -A $@ --run build

.PHONY: veracruz-test
veracruz-test: setup
	$(nix_shell) $(nix_expression) -A $@ --run build

$(now):
	mkdir -p $(dir $@)
	date +%s | tr -d '\n' > $@

.PHONY: test-system-stale
test-system-stale: $(now) # implicit prerequisite: runtime-manager veracruz-server-test veracruz-test test-resources
	$(nix_build) $(nix_expression) -A $@

.PHONY: test-system
test-system: runtime-manager veracruz-server-test veracruz-test test-resources test-system-stale

.PHONY: test-resources
test-resources: setup
	$(MAKE) -C .. sgx test_cases
	cd ../veracruz-server-test && sh populate-test-database.sh
