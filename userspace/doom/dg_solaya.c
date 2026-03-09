#include "doomgeneric.h"
#include "doomkeys.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <unistd.h>
#include <time.h>
#include <termios.h>

/* Embedded doom1.wad via ld -r -b binary */
extern char _binary_doom1_wad_start[];
extern char _binary_doom1_wad_end[];

static int fb_fd = -1;
static struct termios orig_termios;

#define FB_WIDTH  640
#define FB_HEIGHT 480
#define WAD_PATH  "/tmp/doom1.wad"

static void extract_wad(void)
{
    size_t size = _binary_doom1_wad_end - _binary_doom1_wad_start;
    int fd = open(WAD_PATH, O_WRONLY | O_CREAT | O_TRUNC, 0644);
    if (fd < 0) {
        fprintf(stderr, "Failed to create %s\n", WAD_PATH);
        exit(1);
    }
    size_t written = 0;
    while (written < size) {
        ssize_t n = write(fd, _binary_doom1_wad_start + written, size - written);
        if (n <= 0) {
            fprintf(stderr, "Failed to write WAD data\n");
            exit(1);
        }
        written += n;
    }
    close(fd);
    fprintf(stderr, "Extracted doom1.wad (%zu bytes)\n", size);
}

void DG_Init(void)
{
    fb_fd = open("/dev/fb0", O_WRONLY);
    if (fb_fd < 0) {
        fprintf(stderr, "Failed to open /dev/fb0\n");
        exit(1);
    }

    struct termios raw;
    tcgetattr(STDIN_FILENO, &orig_termios);
    raw = orig_termios;
    raw.c_lflag &= ~(ICANON | ECHO);
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 0;
    tcsetattr(STDIN_FILENO, TCSANOW, &raw);

    int flags = fcntl(STDIN_FILENO, F_GETFL, 0);
    fcntl(STDIN_FILENO, F_SETFL, flags | O_NONBLOCK);
}

void DG_DrawFrame(void)
{
    /* Doom renders at DOOMGENERIC_RESX(640) x DOOMGENERIC_RESY(400).
     * Framebuffer is 640x480. Center vertically with 40px offset. */
    static uint32_t fb[FB_WIDTH * FB_HEIGHT];

    int y_offset = (FB_HEIGHT - DOOMGENERIC_RESY) / 2;

    memset(fb, 0, sizeof(fb));
    for (int y = 0; y < DOOMGENERIC_RESY; y++) {
        memcpy(&fb[(y_offset + y) * FB_WIDTH],
               &DG_ScreenBuffer[y * DOOMGENERIC_RESX],
               DOOMGENERIC_RESX * sizeof(uint32_t));
    }

    lseek(fb_fd, 0, SEEK_SET);
    write(fb_fd, fb, sizeof(fb));
}

void DG_SleepMs(uint32_t ms)
{
    struct timespec ts;
    ts.tv_sec = ms / 1000;
    ts.tv_nsec = (ms % 1000) * 1000000L;
    nanosleep(&ts, NULL);
}

uint32_t DG_GetTicksMs(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint32_t)(ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
}

static unsigned char convert_key(unsigned char c)
{
    switch (c) {
        case 'w': return KEY_UPARROW;
        case 's': return KEY_DOWNARROW;
        case 'a': return KEY_LEFTARROW;
        case 'd': return KEY_RIGHTARROW;
        case ' ': return KEY_FIRE;
        case '\n': return KEY_ENTER;
        case 27:  return KEY_ESCAPE;
        case '\t': return KEY_TAB;
        case ',': return KEY_STRAFE_L;
        case '.': return KEY_STRAFE_R;
        case 'e': return KEY_USE;
        default:  return c;
    }
}

static int pending_release = 0;
static unsigned char pending_release_key = 0;

int DG_GetKey(int *pressed, unsigned char *doomKey)
{
    if (pending_release) {
        pending_release = 0;
        *pressed = 0;
        *doomKey = pending_release_key;
        return 1;
    }

    unsigned char c;
    int n = read(STDIN_FILENO, &c, 1);
    if (n <= 0) return 0;

    *pressed = 1;
    *doomKey = convert_key(c);
    pending_release = 1;
    pending_release_key = *doomKey;
    return 1;
}

void DG_SetWindowTitle(const char *title)
{
    (void)title;
}

int main(int argc, char **argv)
{
    extract_wad();

    /* Build argv for doomgeneric: doom -iwad /tmp/doom1.wad [user args...] */
    int new_argc = argc + 2;
    char **new_argv = malloc(sizeof(char *) * (new_argc + 1));
    new_argv[0] = argv[0];
    new_argv[1] = "-iwad";
    new_argv[2] = WAD_PATH;
    for (int i = 1; i < argc; i++)
        new_argv[i + 2] = argv[i];
    new_argv[new_argc] = NULL;

    doomgeneric_Create(new_argc, new_argv);
    for (;;)
        doomgeneric_Tick();
    return 0;
}
