/*
 * gs_fs.h - memory stick helpers shared by the GODsend PSP utilities.
 *
 * Everything here uses the sceIo* user-mode file manager API, so it works on
 * a mounted ms0: without any kernel code.
 */
#ifndef GS_FS_H
#define GS_FS_H

#include <psptypes.h>

#define GS_PATH_MAX 512

/* Recursive byte total and file count under `path`. Returns 0 if unreadable. */
u64 gs_dir_size(const char *path, int *files);

/* True when `path` exists as a directory. */
int gs_is_dir(const char *path);
int gs_is_file(const char *path);

/* Create every missing component of `path` (like mkdir -p). */
int gs_mkdir_p(const char *path);

/* Recursive copy. `dst` is created; parents must exist. */
int gs_copy_tree(const char *src, const char *dst, int *files, u64 *bytes);

/* Human readable size, e.g. "812.4 MB". */
void gs_human_size(u64 bytes, char *out, unsigned int len);

/* Free and total bytes on ms0:. Returns 0 on success. */
int gs_ms0_space(u64 *free_bytes, u64 *total_bytes);

/* Lowercase suffix test, case-insensitive. */
int gs_ends_with_ci(const char *text, const char *suffix);

#endif
