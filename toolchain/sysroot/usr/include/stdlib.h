#pragma once

#include <stddef.h>

void *malloc(size_t size);
void free(void *ptr);
void *calloc(size_t nmemb, size_t size);
void *realloc(void *ptr, size_t size);
void *reallocarray(void *ptr, size_t nmemb, size_t size);

[[noreturn]] void abort(void);
[[noreturn]] void exit(int status);

char *getenv(const char *name);
char *secure_getenv(const char *name);

int atoi(const char *nptr);
long atol(const char *nptr);
long long atoll(const char *nptr);

int abs(int j);
long labs(long j);
long long llabs(long long j);
