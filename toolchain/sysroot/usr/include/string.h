#pragma once

#include <stddef.h>

void *memcpy(void *dest, const void *src, size_t n);
void *memset(void *dest, int count, size_t n);
char *stpcpy(char *dst, const char *src);
char *strcpy(char *dst, const char *src);
char *strcat(char *dst, const char *src);
size_t strlen(const char *s);

char *strchr(const char *s, int c);
char *strrchr(const char *s, int c);
