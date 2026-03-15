OUTPUT_DIR := ../../kernel/compiled_userspace_nix
TARGET := $(OUTPUT_DIR)/doom

CC := riscv64-unknown-linux-musl-gcc
LD := riscv64-unknown-linux-musl-ld
CFLAGS := -static -O3 -DNORMALUNIX -DLINUX -D_DEFAULT_SOURCE -I build
SOLAYA_DIR := $(CURDIR)

SRCS := \
	dummy.c am_map.c doomdef.c doomstat.c dstrings.c \
	d_event.c d_items.c d_iwad.c d_loop.c d_main.c d_mode.c d_net.c \
	f_finale.c f_wipe.c g_game.c hu_lib.c hu_stuff.c info.c \
	i_cdmus.c i_endoom.c i_joystick.c i_scale.c i_sound.c i_system.c \
	i_timer.c memio.c m_argv.c m_bbox.c m_cheat.c m_config.c \
	m_controls.c m_fixed.c m_menu.c m_misc.c m_random.c \
	p_ceilng.c p_doors.c p_enemy.c p_floor.c p_inter.c p_lights.c \
	p_map.c p_maputl.c p_mobj.c p_plats.c p_pspr.c p_saveg.c \
	p_setup.c p_sight.c p_spec.c p_switch.c p_telept.c p_tick.c \
	p_user.c r_bsp.c r_data.c r_draw.c r_main.c r_plane.c r_segs.c \
	r_sky.c r_things.c sha1.c sounds.c statdump.c st_lib.c st_stuff.c \
	s_sound.c tables.c v_video.c wi_stuff.c w_checksum.c w_file.c \
	w_main.c w_wad.c z_zone.c w_file_stdc.c i_input.c i_video.c \
	mus2mid.c doomgeneric.c dg_solaya.c

.PHONY: all clean

all: $(TARGET)

$(TARGET): build/.prepared
	@echo "Compiling doom..."
	@cd build && for f in $(SRCS); do \
		echo "  CC $$f"; \
		$(CC) $(CFLAGS) -c $$f -o $${f%.c}.o || exit 1; \
	done
	mkdir -p build/wad_obj
	cd build && $(LD) -r -b binary -o wad_obj/doom1_wad.o doom1.wad
	cd build && $(CC) $(CFLAGS) -o doom *.o wad_obj/doom1_wad.o -lm
	mkdir -p $(OUTPUT_DIR)
	cp build/doom $@

build/.prepared:
	@test -n "$(DOOMGENERIC_SRC)" || { echo "Error: DOOMGENERIC_SRC not set"; exit 1; }
	@test -n "$(DOOM1_WAD)" || { echo "Error: DOOM1_WAD not set"; exit 1; }
	mkdir -p build
	cp -r $(DOOMGENERIC_SRC)/doomgeneric/* build/
	cp $(SOLAYA_DIR)/dg_solaya.c build/
	cp $(SOLAYA_DIR)/i_video_solaya.c build/i_video.c
	cp $(DOOM1_WAD) build/doom1.wad
	chmod -R u+w build
	touch $@

clean:
	rm -rf build $(TARGET)
