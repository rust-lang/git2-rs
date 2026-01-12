/* Mirrors libgit2's MSVC compatibility typedef.
 * Reference: libgit2/src/util/win32/msvc-compat.h
 */
#ifndef GIT2_SYS_TEST_WIN32_COMPAT_H
#define GIT2_SYS_TEST_WIN32_COMPAT_H

#if defined(_MSC_VER) && !defined(_MODE_T_DEFINED)
typedef unsigned short mode_t;
#define _MODE_T_DEFINED
#endif

#endif
