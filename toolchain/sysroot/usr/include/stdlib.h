#pragma once

#include <stddef.h>

[[noreturn]] void exit(int status);
[[noreturn]] void abort(void);
void free(void *ptr);
void *malloc(size_t size);
void *calloc(size_t nmemb, size_t size);

int atexit(void (*function)(void));
int atoi(const char *nptr);
char *getenv(const char *name);

int abs(int j);
