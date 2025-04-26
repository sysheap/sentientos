#pragma once

#include <stddef.h>

#define SEEK_SET 0

typedef struct {
} FILE;

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

int fflush(FILE *stream);
int fprintf(FILE *stream, const char *format, ...);
int fclose(FILE *stream);
int feof(FILE *stream);
FILE *fopen(const char *pathname, const char *mode);
size_t fread(void *ptr, size_t size, size_t n, FILE *stream);
int fseek(FILE *stream, long int off, int whence);
long int ftell(FILE *stream);
size_t fwrite(const void *ptr, size_t size, size_t n, FILE *s);

void setbuf(FILE *stream, char *buf);
int vfprintf(FILE *stream, const char *format, __builtin_va_list ap);
int sprintf(char *s, const char *format, ...);
int printf(const char *format, ...);
int puts(const char *s);
