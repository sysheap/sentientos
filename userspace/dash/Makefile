OUTPUT_DIR := ../../kernel/compiled_userspace_nix
TARGETS := $(OUTPUT_DIR)/dash $(OUTPUT_DIR)/sh

CC := riscv64-unknown-linux-musl-gcc
CFLAGS :=
LDFLAGS := -static

.PHONY: all clean

all: $(TARGETS)

$(OUTPUT_DIR)/dash: build/src/dash
	mkdir -p $(OUTPUT_DIR)
	cp $< $@

$(OUTPUT_DIR)/sh: $(OUTPUT_DIR)/dash
	ln -sf dash $@

build/src/dash: build/configure
	cd build && ./configure --host=riscv64-unknown-linux-musl CC="$(CC)" LDFLAGS="$(LDFLAGS)"
	$(MAKE) -C build

build/configure:
	@test -n "$(DASH_SRC)" || { echo "Error: DASH_SRC not set"; exit 1; }
	mkdir -p build
	cp -r $(DASH_SRC)/* build/
	chmod -R u+w build
	cd build && touch aclocal.m4 configure config.h.in Makefile.in src/Makefile.in

clean:
	rm -rf build $(TARGETS)
