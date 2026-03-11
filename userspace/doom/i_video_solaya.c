// Optimized i_video.c for Solaya — pre-computed RGBA LUT + memcpy scaled rows
// Based on doomgeneric i_video.c, Copyright (C) 1993-1996 by id Software, Inc.
// GPL v2+

static const char
rcsid[] = "$Id: i_x.c,v 1.6 1997/02/03 22:45:10 b1 Exp $";

#include "config.h"
#include "v_video.h"
#include "m_argv.h"
#include "d_event.h"
#include "d_main.h"
#include "i_video.h"
#include "i_system.h"
#include "z_zone.h"

#include "tables.h"
#include "doomkeys.h"

#include "doomgeneric.h"

#include <stdbool.h>
#include <stdlib.h>
#include <fcntl.h>
#include <stdarg.h>
#include <sys/types.h>
#include <string.h>

struct FB_BitField
{
	uint32_t offset;
	uint32_t length;
};

struct FB_ScreenInfo
{
	uint32_t xres;
	uint32_t yres;
	uint32_t xres_virtual;
	uint32_t yres_virtual;
	uint32_t bits_per_pixel;
	struct FB_BitField red;
	struct FB_BitField green;
	struct FB_BitField blue;
	struct FB_BitField transp;
};

static struct FB_ScreenInfo s_Fb;
int fb_scaling = 1;
int usemouse = 0;

static struct color colors[256];
static uint32_t rgba_lut[256];

void I_GetEvent(void);

byte *I_VideoBuffer = NULL;
boolean screensaver_mode = false;
boolean screenvisible;
float mouse_acceleration = 2.0;
int mouse_threshold = 10;
int usegamma = 0;

typedef struct
{
	byte r;
	byte g;
	byte b;
} col_t;

static uint16_t rgb565_palette[256];

void cmap_to_rgb565(uint16_t * out, uint8_t * in, int in_pixels)
{
    int i, j;
    struct color c;
    uint16_t r, g, b;

    for (i = 0; i < in_pixels; i++)
    {
        c = colors[*in];
        r = ((uint16_t)(c.r >> 3)) << 11;
        g = ((uint16_t)(c.g >> 2)) << 5;
        b = ((uint16_t)(c.b >> 3)) << 0;
        *out = (r | g | b);

        in++;
        for (j = 0; j < fb_scaling; j++) {
            out++;
        }
    }
}

void cmap_to_fb(uint8_t *out, uint8_t *in, int in_pixels)
{
    int i, k;
    uint32_t pix;
    uint32_t *out32 = (uint32_t *)out;

    for (i = 0; i < in_pixels; i++)
    {
        pix = rgba_lut[*in];
        for (k = 0; k < fb_scaling; k++) {
            *out32++ = pix;
        }
        in++;
    }
}

void I_InitGraphics (void)
{
    int i;
    char *mode;

	memset(&s_Fb, 0, sizeof(struct FB_ScreenInfo));
	s_Fb.xres = DOOMGENERIC_RESX;
	s_Fb.yres = DOOMGENERIC_RESY;
	s_Fb.xres_virtual = s_Fb.xres;
	s_Fb.yres_virtual = s_Fb.yres;

	int gfxmodeparm = M_CheckParmWithArgs("-gfxmode", 1);

	if (gfxmodeparm) {
		mode = myargv[gfxmodeparm + 1];
	}
	else {
		mode = "rgba8888";
	}

	if (strcmp(mode, "rgba8888") == 0) {
		s_Fb.bits_per_pixel = 32;
		s_Fb.blue.length = 8;
		s_Fb.green.length = 8;
		s_Fb.red.length = 8;
		s_Fb.transp.length = 8;
		s_Fb.blue.offset = 0;
		s_Fb.green.offset = 8;
		s_Fb.red.offset = 16;
		s_Fb.transp.offset = 24;
	}
	else if (strcmp(mode, "rgb565") == 0) {
		s_Fb.bits_per_pixel = 16;
		s_Fb.blue.length = 5;
		s_Fb.green.length = 6;
		s_Fb.red.length = 5;
		s_Fb.transp.length = 0;
		s_Fb.blue.offset = 11;
		s_Fb.green.offset = 5;
		s_Fb.red.offset = 0;
		s_Fb.transp.offset = 16;
	}
	else
		I_Error("Unknown gfxmode value: %s\n", mode);

    printf("I_InitGraphics: framebuffer: x_res: %d, y_res: %d, bpp: %d\n",
            s_Fb.xres, s_Fb.yres, s_Fb.bits_per_pixel);

    printf("I_InitGraphics: DOOM screen size: w x h: %d x %d\n", SCREENWIDTH, SCREENHEIGHT);

    i = M_CheckParmWithArgs("-scaling", 1);
    if (i > 0) {
        i = atoi(myargv[i + 1]);
        fb_scaling = i;
        printf("I_InitGraphics: Scaling factor: %d\n", fb_scaling);
    } else {
        fb_scaling = s_Fb.xres / SCREENWIDTH;
        if (s_Fb.yres / SCREENHEIGHT < fb_scaling)
            fb_scaling = s_Fb.yres / SCREENHEIGHT;
        printf("I_InitGraphics: Auto-scaling factor: %d\n", fb_scaling);
    }

	I_VideoBuffer = (byte*)Z_Malloc (SCREENWIDTH * SCREENHEIGHT, PU_STATIC, NULL);
	screenvisible = true;

    extern void I_InitInput(void);
    I_InitInput();
}

void I_ShutdownGraphics (void)
{
	Z_Free (I_VideoBuffer);
}

void I_StartFrame (void)
{
}

void I_StartTic (void)
{
	I_GetEvent();
}

void I_UpdateNoBlit (void)
{
}

void I_FinishUpdate (void)
{
    int y;
    unsigned char *line_in, *line_out;

    int x_offset     = (((s_Fb.xres - (SCREENWIDTH  * fb_scaling)) * s_Fb.bits_per_pixel/8)) / 2;
    int x_offset_end = ((s_Fb.xres - (SCREENWIDTH  * fb_scaling)) * s_Fb.bits_per_pixel/8) - x_offset;
    int row_bytes    = SCREENWIDTH * fb_scaling * (s_Fb.bits_per_pixel / 8);

    line_in  = (unsigned char *) I_VideoBuffer;
    line_out = (unsigned char *) DG_ScreenBuffer;

    y = SCREENHEIGHT;

    while (y--)
    {
        /* Convert first scaled row */
        line_out += x_offset;
        cmap_to_fb(line_out, line_in, SCREENWIDTH);
        unsigned char *first_row = line_out;
        line_out += row_bytes + x_offset_end;

        /* Memcpy remaining scaled rows instead of re-converting */
        int i;
        for (i = 1; i < fb_scaling; i++) {
            line_out += x_offset;
            memcpy(line_out, first_row, row_bytes);
            line_out += row_bytes + x_offset_end;
        }
        line_in += SCREENWIDTH;
    }

	DG_DrawFrame();
}

void I_ReadScreen (byte* scr)
{
    memcpy (scr, I_VideoBuffer, SCREENWIDTH * SCREENHEIGHT);
}

#define GFX_RGB565(r, g, b)			((((r & 0xF8) >> 3) << 11) | (((g & 0xFC) >> 2) << 5) | ((b & 0xF8) >> 3))
#define GFX_RGB565_R(color)			((0xF800 & color) >> 11)
#define GFX_RGB565_G(color)			((0x07E0 & color) >> 5)
#define GFX_RGB565_B(color)			(0x001F & color)

void I_SetPalette (byte* palette)
{
    int i;

    for (i=0; i<256; ++i ) {
        colors[i].a = 0;
        colors[i].r = gammatable[usegamma][*palette++];
        colors[i].g = gammatable[usegamma][*palette++];
        colors[i].b = gammatable[usegamma][*palette++];

        /* Pre-compute RGBA pixel for 32bpp framebuffer */
        rgba_lut[i] = ((uint32_t)colors[i].r << s_Fb.red.offset) |
                      ((uint32_t)colors[i].g << s_Fb.green.offset) |
                      ((uint32_t)colors[i].b << s_Fb.blue.offset);
    }
}

int I_GetPaletteIndex (int r, int g, int b)
{
    int best, best_diff, diff;
    int i;
    col_t color;

    best = 0;
    best_diff = INT_MAX;

    for (i = 0; i < 256; ++i)
    {
    	color.r = GFX_RGB565_R(rgb565_palette[i]);
    	color.g = GFX_RGB565_G(rgb565_palette[i]);
    	color.b = GFX_RGB565_B(rgb565_palette[i]);

        diff = (r - color.r) * (r - color.r)
             + (g - color.g) * (g - color.g)
             + (b - color.b) * (b - color.b);

        if (diff < best_diff)
        {
            best = i;
            best_diff = diff;
        }

        if (diff == 0)
        {
            break;
        }
    }

    return best;
}

void I_BeginRead (void)
{
}

void I_EndRead (void)
{
}

void I_SetWindowTitle (char *title)
{
	DG_SetWindowTitle(title);
}

void I_GraphicsCheckCommandLine (void)
{
}

void I_SetGrabMouseCallback (grabmouse_callback_t func)
{
}

void I_EnableLoadingDisk(void)
{
}

void I_BindVideoVariables (void)
{
}

void I_DisplayFPSDots (boolean dots_on)
{
}

void I_CheckIsScreensaver (void)
{
}
