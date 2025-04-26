#pragma once

#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>

#define PROT_READ 0x01
#define PROT_WRITE 0x02

#define MAP_SHARED 0x01
#define MAP_PRIVATE 0x02
#define MAP_ANONYMOUS 0x4

void *mmap(void *addr, size_t length, int prot, int flags, int fd,
           off_t offset);
int munmap(void *addr, size_t length);
