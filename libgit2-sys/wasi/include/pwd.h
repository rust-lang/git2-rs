/*
 * Minimal pwd.h for WASI, whose libc has none. libgit2 includes it from
 * sysdir.c and only calls getpwuid_r when running setuid or in a macOS
 * sandbox, neither of which happens with compat.c's identity stubs, so this
 * just needs to compile; it reports no entry if ever reached.
 */
#ifndef LIBGIT2_SYS_WASI_PWD_H
#define LIBGIT2_SYS_WASI_PWD_H

#include <stddef.h>
#include <sys/types.h>

struct passwd {
	char *pw_name;
	char *pw_passwd;
	uid_t pw_uid;
	gid_t pw_gid;
	char *pw_gecos;
	char *pw_dir;
	char *pw_shell;
};

static inline int getpwuid_r(uid_t uid, struct passwd *pwd, char *buf,
	size_t buflen, struct passwd **result)
{
	(void)uid; (void)pwd; (void)buf; (void)buflen;
	*result = NULL;
	return 0;
}

#endif
