#include "gs_fs.h"

#include <pspiofilemgr.h>
#include <pspiofilemgr_devctl.h>
#include <pspiofilemgr_stat.h>
#include <pspkernel.h>

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>

static int gs_is_dir_mode(unsigned int mode)
{
	return (mode & FIO_S_IFMT) == FIO_S_IFDIR;
}

int gs_is_dir(const char *path)
{
	SceIoStat st;
	memset(&st, 0, sizeof(st));
	if (sceIoGetstat(path, &st) < 0) {
		return 0;
	}
	return gs_is_dir_mode(st.st_mode);
}

int gs_is_file(const char *path)
{
	SceIoStat st;
	memset(&st, 0, sizeof(st));
	if (sceIoGetstat(path, &st) < 0) {
		return 0;
	}
	return (st.st_mode & FIO_S_IFMT) == FIO_S_IFREG;
}

u64 gs_dir_size(const char *path, int *files)
{
	SceIoDirent entry;
	int fd;
	u64 total = 0;
	int count = 0;

	if (files != NULL) {
		*files = 0;
	}
	fd = sceIoDopen(path);
	if (fd < 0) {
		return 0;
	}
	for (;;) {
		memset(&entry, 0, sizeof(entry));
		if (sceIoDread(fd, &entry) <= 0) {
			break;
		}
		if (strcmp(entry.d_name, ".") == 0 || strcmp(entry.d_name, "..") == 0) {
			continue;
		}
		{
			char child[GS_PATH_MAX];
			snprintf(child, sizeof(child), "%s/%s", path, entry.d_name);
			if (gs_is_dir_mode(entry.d_stat.st_mode)) {
				int nested = 0;
				total += gs_dir_size(child, &nested);
				count += nested;
			} else if ((entry.d_stat.st_mode & FIO_S_IFMT) == FIO_S_IFREG) {
				total += (u64)entry.d_stat.st_size;
				count++;
			}
		}
	}
	sceIoDclose(fd);
	if (files != NULL) {
		*files = count;
	}
	return total;
}

int gs_mkdir_p(const char *path)
{
	char work[GS_PATH_MAX];
	size_t i;
	size_t len;

	if (path == NULL) {
		return -1;
	}
	len = strlen(path);
	if (len == 0 || len >= sizeof(work)) {
		return -1;
	}
	strcpy(work, path);
	for (i = 1; i < len; i++) {
		if (work[i] == '/') {
			work[i] = '\0';
			/* Drive roots such as "ms0:" cannot be created. */
			if (work[i - 1] != ':' && strlen(work) > 4) {
				sceIoMkdir(work, 0777);
			}
			work[i] = '/';
		}
	}
	if (len > 4) {
		sceIoMkdir(work, 0777);
	}
	return gs_is_dir(path) ? 0 : -1;
}

static int gs_copy_file(const char *src, const char *dst)
{
	static unsigned char buffer[16 * 1024];
	SceUID in;
	SceUID out;
	int error = 0;

	in = sceIoOpen(src, PSP_O_RDONLY, 0);
	if (in < 0) {
		return (int)in;
	}
	out = sceIoOpen(dst, PSP_O_WRONLY | PSP_O_CREAT | PSP_O_TRUNC, 0777);
	if (out < 0) {
		sceIoClose(in);
		return (int)out;
	}
	for (;;) {
		int read = sceIoRead(in, buffer, sizeof(buffer));
		if (read < 0) {
			error = read;
			break;
		}
		if (read == 0) {
			break;
		}
		if (sceIoWrite(out, buffer, read) != read) {
			error = -1;
			break;
		}
	}
	sceIoClose(out);
	sceIoClose(in);
	return error;
}

int gs_copy_tree(const char *src, const char *dst, int *files, u64 *bytes)
{
	SceIoDirent entry;
	int fd;
	int error = 0;
	int count = 0;
	u64 total = 0;

	if (files != NULL) {
		*files = 0;
	}
	if (bytes != NULL) {
		*bytes = 0;
	}
	if (sceIoMkdir(dst, 0777) < 0 && !gs_is_dir(dst)) {
		return -1;
	}
	fd = sceIoDopen(src);
	if (fd < 0) {
		return (int)fd;
	}
	for (;;) {
		memset(&entry, 0, sizeof(entry));
		if (sceIoDread(fd, &entry) <= 0) {
			break;
		}
		if (strcmp(entry.d_name, ".") == 0 || strcmp(entry.d_name, "..") == 0) {
			continue;
		}
		{
			char from[GS_PATH_MAX];
			char to[GS_PATH_MAX];
			snprintf(from, sizeof(from), "%s/%s", src, entry.d_name);
			snprintf(to, sizeof(to), "%s/%s", dst, entry.d_name);
			if (gs_is_dir_mode(entry.d_stat.st_mode)) {
				int nested_files = 0;
				u64 nested_bytes = 0;
				int result = gs_copy_tree(from, to, &nested_files, &nested_bytes);
				count += nested_files;
				total += nested_bytes;
				if (result < 0 && error == 0) {
					error = result;
				}
			} else if ((entry.d_stat.st_mode & FIO_S_IFMT) == FIO_S_IFREG) {
				int result = gs_copy_file(from, to);
				if (result < 0) {
					if (error == 0) {
						error = result;
					}
				} else {
					count++;
					total += (u64)entry.d_stat.st_size;
				}
			}
		}
	}
	sceIoDclose(fd);
	if (files != NULL) {
		*files = count;
	}
	if (bytes != NULL) {
		*bytes = total;
	}
	return error;
}

void gs_human_size(u64 bytes, char *out, unsigned int len)
{
	double value;
	const char *unit;

	if (bytes >= (u64)1024 * 1024 * 1024) {
		value = (double)bytes / (1024.0 * 1024.0 * 1024.0);
		unit = "GB";
	} else if (bytes >= (u64)1024 * 1024) {
		value = (double)bytes / (1024.0 * 1024.0);
		unit = "MB";
	} else if (bytes >= (u64)1024) {
		value = (double)bytes / 1024.0;
		unit = "KB";
	} else {
		snprintf(out, len, "%llu B", (unsigned long long)bytes);
		return;
	}
	snprintf(out, len, "%.1f %s", value, unit);
}

int gs_ms0_space(u64 *free_bytes, u64 *total_bytes)
{
	SceDevInf info;
	u64 cluster;

	if (free_bytes != NULL) {
		*free_bytes = 0;
	}
	if (total_bytes != NULL) {
		*total_bytes = 0;
	}
	memset(&info, 0, sizeof(info));
	if (sceIoDevctl("ms0:", SCE_PR_GETDEV, NULL, 0, &info, sizeof(info)) < 0) {
		return -1;
	}
	cluster = (u64)(u32)info.sectorSize * (u64)(u32)info.sectorCount;
	if (free_bytes != NULL) {
		*free_bytes = cluster * (u64)(u32)info.freeClusters;
	}
	if (total_bytes != NULL) {
		*total_bytes = cluster * (u64)(u32)info.maxClusters;
	}
	return 0;
}

int gs_ends_with_ci(const char *text, const char *suffix)
{
	size_t tlen;
	size_t slen;

	if (text == NULL || suffix == NULL) {
		return 0;
	}
	tlen = strlen(text);
	slen = strlen(suffix);
	if (slen == 0 || tlen < slen) {
		return 0;
	}
	return strcasecmp(text + tlen - slen, suffix) == 0;
}
