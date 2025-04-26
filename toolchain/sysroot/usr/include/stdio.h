#pragma once

#include <stdarg.h>
#include <stddef.h>
#include <sys/types.h>

typedef struct file {

} FILE;

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

#define SEEK_SET 0x01

int fflush(FILE *stream);

size_t fread(void *ptr, size_t size, size_t nmemb, FILE *stream);
size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream);

void clearerr(FILE *stream);
int feof(FILE *stream);
int ferror(FILE *stream);

FILE *fopen(const char *pathname, const char *mode);
FILE *fdopen(int fd, const char *mode);
FILE *freopen(const char *pathname, const char *mode, FILE *stream);

int fseek(FILE *stream, long offset, int whence);
long ftell(FILE *stream);

void rewind(FILE *stream);

int fgetpos(FILE *stream, fpos_t *pos);
int fsetpos(FILE *stream, const fpos_t *pos);

int fclose(FILE *stream);

int printf(const char *format, ...);
int fprintf(FILE *stream, const char *format, ...);
int dprintf(int fd, const char *format, ...);
int sprintf(char *str, const char *format, ...);
int snprintf(char *str, size_t size, const char *format, ...);

int vprintf(const char *format, va_list ap);
int vfprintf(FILE *stream, const char *format, va_list ap);
int vdprintf(int fd, const char *format, va_list ap);
int vsprintf(char *str, const char *format, va_list ap);
int vsnprintf(char *str, size_t size, const char *format, va_list ap);

int puts(const char *s);
