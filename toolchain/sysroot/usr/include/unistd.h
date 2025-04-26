#pragma once

#include <stdint.h>
#include <sys/types.h>

pid_t fork(void);
int execv(const char *path, char *const argv[]);
int execvp(const char *file, char *const argv[]);
int execve(const char *path, char *const argv[], char *const envp[]);
pid_t getpid(void);
