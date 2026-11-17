/*
 * Force-included into every libgit2 translation unit when targeting WASI.
 * wasi-libc omits these POSIX functions (libgit2 calls them to seed its PRNG,
 * check file ownership and find the home directory); compat.c defines them.
 * getpid is declared by wasi-libc itself under _WASI_EMULATED_GETPID.
 */
#ifndef LIBGIT2_SYS_WASI_COMPAT_H
#define LIBGIT2_SYS_WASI_COMPAT_H

#include <sys/types.h>

uid_t getuid(void);
uid_t geteuid(void);
gid_t getgid(void);
pid_t getppid(void);
pid_t getpgid(pid_t pid);
pid_t getsid(pid_t pid);

#endif
