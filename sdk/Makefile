# Top-level build-all Makefile for the SDK
#
# Builds:
#   - freestanding-execution-engine
#   - all of the Rust examples in the rust-language-support/examples/ directory
#   - all of the datasets in data-generators/ directory
#
# AUTHORS
#
# The Veracruz Development Team.
#
# COPYRIGHT
#
# See the `LICENSE_MIT.markdown` file in the Veracruz root directory for licensing
# and copyright information.

# HACK temporary workaround for CI running out of disk space
export CARGO_TARGET_DIR := $(abspath target)

# All programs
RUST_EXAMPLES = idash2017-logistic-regression \
	   	   linear-regression \
		   moving-average-convergence-divergence \
		   private-set-intersection \
		   random-source \
		   intersection-set-sum \
		   logistic-regression \
		   nop \
		   private-set-intersection-sum \
		   string-edit-distance \
		   number-stream-accumulation \
		   read-file \
		   image-processing \
		   shamir-secret-sharing
RUST_EXAMPLE_DIR = rust-examples/
# Navigate to the actual wasm programs.
RELATIVE_RUST_EXAMPLES = $(foreach EXAMPLE,$(RUST_EXAMPLES),$(CARGO_TARGET_DIR)/wasm32-wasi/release/$(EXAMPLE).wasm)
# All dataset filenames
DATA = hello-world-1.dat \
	   hello-world-2.dat \
	   intersection-advertisement-viewer.dat \
	   intersection-customer.dat \
	   linear-regression.dat \
	   private-set-1.dat \
	   private-set-2.dat \
	   number-stream-1.dat \
	   number-stream-2.dat \
	   number-stream-init.dat \
	   share-1.dat \
	   share-2.dat \
	   share-3.dat \
	   idash2017/ \
	   macd/ \
	   private-set-inter-sum/ \
	   image-processing.png
DATASET_PATH = datasets/
RELATIVE_DATA = $(addprefix $(DATASET_PATH), $(DATA))
# All data generators for `fmt` and `clean` targets.
DATA_GENERATORS = idash2017-logistic-regression-generator \
	              intersection-set-sum-generator \
	              moving-average-convergence-divergence-generator \
	              private-set-intersection-generator \
	              private-set-intersection-sum-generator \
	              string-generator \
	              linear-regression-generator \
	              number-stream-generator \
	              image-processing-generator \
	              shamir-secret-sharing
DATA_GENERATORS_PATH = data-generators
RELATIVE_DATA_GEN = $(addprefix $(DATA_GENERATORS_PATH)/, $(DATA_GENERATORS))

.PHONY: all clean clean-cargo-lock doc fmt fmt-check freestanding-execution-engine wasm-checker env update

all: wasm-checker $(RELATIVE_DATA) $(RELATIVE_RUST_EXAMPLES) freestanding-execution-engine

wasm-checker:
	$(MAKE) -C wasm-checker

.SECONDEXPANSION:
$(RELATIVE_RUST_EXAMPLES): %.wasm : $$(addsuffix /src/main.rs,$$(addprefix $(RUST_EXAMPLE_DIR),$$(basename $$(notdir $$@)))) env wasm-checker
	$(MAKE) -C $(addprefix $(RUST_EXAMPLE_DIR),$(basename $(notdir $@)))
	./wasm-checker/bin/wasm-checker -c ./wasm-checker/wasm-checker-config.json $@

freestanding-execution-engine: env
	$(MAKE) -C freestanding-execution-engine/

# Specify the dependency for all datasets
$(addprefix $(DATASET_PATH),hello-world-1.dat hello-world-2.dat): %.dat : $$(addprefix $(DATA_GENERATORS_PATH)/string-generator/,Makefile src/main.rs $$(notdir %.txt))
$(addprefix $(DATASET_PATH),intersection-advertisement-viewer.dat intersection-advertisement-customer.dat): %.dat : $$(addprefix $(DATA_GENERATORS_PATH)/intersection-set-sum-generator/,Makefile src/main.rs)
$(DATASET_PATH)linear-regression.dat: $$(addprefix $(DATA_GENERATORS_PATH)/linear-regression-generator/,Makefile src/main.rs)
$(addprefix $(DATASET_PATH),private-set-1.dat private-set-2.dat): %.dat : $$(addprefix $(DATA_GENERATORS_PATH)/private-set-intersection-generator/,Makefile src/main.rs $$(notdir %.csv))
$(addprefix $(DATASET_PATH),share-1.dat share-2.dat share-3.dat): $(addprefix $(DATA_GENERATORS_PATH)/shamir-secret-sharing/,Makefile src/main.rs)
$(DATASET_PATH)idash2017/: $$(addprefix $(DATA_GENERATORS_PATH)/idash2017-logistic-regression-generator/,Makefile src/main.rs)
$(DATASET_PATH)macd/: $$(addprefix $(DATA_GENERATORS_PATH)/moving-average-convergence-divergence-generator/,Makefile src/main.rs)
$(DATASET_PATH)private-set-inter-sum/: $$(addprefix $(DATA_GENERATORS_PATH)/private-set-intersection-sum-generator/,Makefile src/main.rs)
$(addprefix $(DATASET_PATH),number-stream-1.dat number-stream-2.dat number-stream-init.dat): %.dat : $$(addprefix $(DATA_GENERATORS_PATH)/number-stream-generator/,Makefile src/main.rs)
$(addprefix $(DATASET_PATH),image-processing.png): %.png : $$(addprefix $(DATA_GENERATORS_PATH)/image-processing-generator/,Makefile src/main.rs)

# Call makefiles to generate datasets
$(RELATIVE_DATA):
	$(MAKE) -C $(dir $<)

env:
# Don't use rustup if we are building in a Nix shell
	rustup target add wasm32-wasi
	rustup component add rust-src rust-std

fmt:
	for data in $(RELATIVE_DATA_GEN); do \
		$(MAKE) -C $$data fmt; \
	done
	for example in $(addprefix $(RUST_EXAMPLE_DIR),$(basename $(notdir $(RELATIVE_RUST_EXAMPLES)))); do \
		$(MAKE) -C $$example fmt; \
	done
	$(MAKE) -C freestanding-execution-engine/ fmt

fmt-check:
	for data in $(RELATIVE_DATA_GEN); do \
                $(MAKE) -C $$data fmt-check; \
        done
	for example in $(addprefix $(RUST_EXAMPLE_DIR),$(basename $(notdir $(RELATIVE_RUST_EXAMPLES)))); do \
                $(MAKE) -C $$example fmt-check; \
        done
	$(MAKE) -C freestanding-execution-engine/ fmt-check

doc:
	for example in $(addprefix $(RUST_EXAMPLE_DIR),$(basename $(notdir $(RELATIVE_RUST_EXAMPLES)))); do \
                $(MAKE) -C $$example doc; \
        done
	$(MAKE) -C freestanding-execution-engine/ doc

clean:
	for data in $(RELATIVE_DATA_GEN); do \
		$(MAKE) -C $$data clean; \
	done
	rm -rf datasets
	for example in $(addprefix $(RUST_EXAMPLE_DIR),$(basename $(notdir $(RELATIVE_RUST_EXAMPLES)))); do \
		$(MAKE) -C $$example clean; \
	done
	$(MAKE) -C freestanding-execution-engine/ clean
	$(MAKE) -C wasm-checker/ clean

clean-cargo-lock:
	$(MAKE) -C freestanding-execution-engine clean-cargo-lock
	for data in $(RELATIVE_DATA_GEN); do \
                rm -f $(addprefix ./$$data/, Cargo.lock); \
        done
	for example in $(addprefix $(RUST_EXAMPLE_DIR),$(basename $(notdir $(RELATIVE_RUST_EXAMPLES)))); do \
		rm -f $(addprefix ./$$example/, Cargo.lock); \
        done

quick-clean:
	for data in $(RELATIVE_DATA_GEN); do \
		$(MAKE) -C $$data quick-clean; \
	done
	rm -rf datasets
	for example in $(addprefix $(RUST_EXAMPLE_DIR),$(basename $(notdir $(RELATIVE_RUST_EXAMPLES)))); do \
		$(MAKE) -C $$example quick-clean; \
	done
	$(MAKE) -C freestanding-execution-engine/ quick-clean

update:
	for data in $(RELATIVE_DATA_GEN); do \
		cd $(addprefix $(shell pwd)/,$$data) && cargo update; \
        done
	for example in $(addprefix $(RUST_EXAMPLE_DIR),$(basename $(notdir $(RELATIVE_RUST_EXAMPLES)))); do \
		cd $(addprefix $(shell pwd)/,$$example) && cargo update; \
        done
	$(MAKE) -C freestanding-execution-engine/ update
