/*
 * Definitions for symbols libgit2 references that have no WASI preview 1
 * equivalent. There is a single user (uid 0) and a single process, and no
 * sockets.
 */
#include <sys/types.h>

#include "git2_util.h"
#include "streams/socket.h"

uid_t getuid(void)
{
	return 0;
}

uid_t geteuid(void)
{
	return 0;
}

gid_t getgid(void)
{
	return 0;
}

pid_t getpid(void)
{
	return 1;
}

pid_t getppid(void)
{
	return 0;
}

pid_t getpgid(pid_t pid)
{
	GIT_UNUSED(pid);
	return 1;
}

pid_t getsid(pid_t pid)
{
	GIT_UNUSED(pid);
	return 1;
}

/* Replaces streams/socket.c: settable through git_libgit2_opts, unused. */
int git_socket_stream__connect_timeout = 0;
int git_socket_stream__timeout = 0;

int git_socket_stream_global_init(void)
{
	return 0;
}

int git_socket_stream_new(git_stream **out, const char *host, const char *port)
{
	GIT_UNUSED(host);
	GIT_UNUSED(port);
	*out = NULL;
	git_error_set(GIT_ERROR_NET, "sockets are not supported on WASI");
	return -1;
}
