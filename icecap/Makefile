crates_dir := crates
build_dir := build
nix_root := nix-root

now := $(build_dir)/NOW

nix_binary_cache_url := http://52.214.93.220:5000
nix_binary_cache_public_key := icecap:j+jQQU4VWcGmre43aPtCt1GNfLmtO2IMKoZ1MsHOmVY=
nix_binary_cache_flags := \
	--option extra-substituters $(nix_binary_cache_url) \
	--option extra-trusted-public-keys $(nix_binary_cache_public_key)

nix_build := nix-build $(nix_binary_cache_flags) -j$(shell nproc)
nix_shell := NIX_BUILD_SHELL=bash nix-shell $(nix_binary_cache_flags) -j$(shell nproc) --pure
nix_expression := targets.nix

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

.PHONY: test-resources
test-resources:
	$(MAKE) -C ../sdk
	TEE=icecap $(MAKE) -C ../test-collateral

.PHONY: test-system
test-system: $(now) runtime-manager veracruz-server-test veracruz-test test-resources
	$(nix_build) $(nix_expression) -A $@

# useful shortcut
.PHONY: test-system-stale
test-system-stale: $(now) # implicit prerequisites: runtime-manager veracruz-server-test veracruz-test test-resources
	$(nix_build) $(nix_expression) -A test-system

# HACK for setting time in host and realm
$(now):
	mkdir -p $(dir $@)
	date +%s | tr -d '\n' > $@
