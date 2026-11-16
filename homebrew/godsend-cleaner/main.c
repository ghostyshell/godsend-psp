/*
 * godsend-cleaner - memory stick storage cleaner for the PSP.
 *
 * Shows free/used space on ms0:, the biggest folders under PSP/ and the bytes
 * a cleanup would reclaim (PSP/COMMON cache files plus zero-byte and *.tmp
 * leftovers anywhere under PSP/ except SAVEDATA). Cross confirms the delete.
 *
 * SAVEDATA contents are never walked for deletion.
 */
#include <pspkernel.h>
#include <pspiofilemgr.h>

#include <stdio.h>
#include <string.h>

#include "gs_fs.h"
#include "gs_ui.h"

PSP_MODULE_INFO("GODsend Cleaner", 0, 1, 1);
PSP_MAIN_THREAD_ATTR(PSP_THREAD_ATTR_USER | PSP_THREAD_ATTR_VFPU);

#define PSP_ROOT "ms0:/PSP"

struct folder_row {
	const char *name;
	u64 bytes;
	int files;
};

static const char *const FOLDERS[] = {
	"SAVEDATA", "GAME", "PICTURE", "MUSIC", "VIDEO", "COMMON",
};

static int is_temp_name(const char *name)
{
	return gs_ends_with_ci(name, ".tmp");
}

/*
 * Walk one tree. `in_common` marks files below PSP/COMMON (all of them are
 * cache). Everything else counts when it is empty or ends in .tmp, except
 * inside SAVEDATA which is skipped outright.
 */
static void walk(const char *dir, int in_common, int do_delete, int *count, u64 *bytes)
{
	SceIoDirent entry;
	int fd;

	fd = sceIoDopen(dir);
	if (fd < 0) {
		return;
	}
	for (;;) {
		char path[GS_PATH_MAX];
		memset(&entry, 0, sizeof(entry));
		if (sceIoDread(fd, &entry) <= 0) {
			break;
		}
		if (strcmp(entry.d_name, ".") == 0 || strcmp(entry.d_name, "..") == 0) {
			continue;
		}
		snprintf(path, sizeof(path), "%s/%s", dir, entry.d_name);
		if ((entry.d_stat.st_mode & FIO_S_IFMT) == FIO_S_IFDIR) {
			if (strcmp(entry.d_name, "SAVEDATA") == 0) {
				continue;
			}
			walk(path, in_common || strcmp(entry.d_name, "COMMON") == 0, do_delete, count, bytes);
			continue;
		}
		if ((entry.d_stat.st_mode & FIO_S_IFMT) != FIO_S_IFREG) {
			continue;
		}
		if (!(in_common || entry.d_stat.st_size == 0 || is_temp_name(entry.d_name))) {
			continue;
		}
		if (do_delete) {
			if (sceIoRemove(path) < 0) {
				continue;
			}
		}
		(*count)++;
		*bytes += (u64)entry.d_stat.st_size;
	}
	sceIoDclose(fd);
}

static void draw_report(void)
{
	u64 free_bytes = 0;
	u64 total_bytes = 0;
	u64 used_bytes = 0;
	u64 reclaim_bytes = 0;
	int reclaim_files = 0;
	u64 after_bytes = 0;
	int i;
	struct folder_row rows[6];
	int row_count = 0;
	char free_text[32];
	char used_text[32];
	char total_text[32];
	char reclaim_text[32];
	char after_text[32];

	gs_reset("GODsend Cleaner");

	if (gs_ms0_space(&free_bytes, &total_bytes) < 0) {
		gs_color(GS_COLOR_BAD);
		gs_printf("\nms0: did not report its capacity.\n");
		gs_color(GS_COLOR_DIM);
		gs_printf("Is the memory stick mounted?\n");
		return;
	}
	used_bytes = total_bytes > free_bytes ? total_bytes - free_bytes : 0;
	gs_human_size(free_bytes, free_text, sizeof(free_text));
	gs_human_size(used_bytes, used_text, sizeof(used_text));
	gs_human_size(total_bytes, total_text, sizeof(total_text));
	gs_printf("\nms0:  %s free of %s   (%s used)\n", free_text, total_text, used_text);

	for (i = 0; i < (int)(sizeof(FOLDERS) / sizeof(FOLDERS[0])); i++) {
		char path[GS_PATH_MAX];
		snprintf(path, sizeof(path), "%s/%s", PSP_ROOT, FOLDERS[i]);
		if (!gs_is_dir(path)) {
			continue;
		}
		rows[row_count].name = FOLDERS[i];
		rows[row_count].files = 0;
		rows[row_count].bytes = gs_dir_size(path, &rows[row_count].files);
		row_count++;
	}

	/* Biggest first, plain selection sort over six rows. */
	for (i = 0; i < row_count; i++) {
		int best = i;
		int j;
		for (j = i + 1; j < row_count; j++) {
			if (rows[j].bytes > rows[best].bytes) {
				best = j;
			}
		}
		if (best != i) {
			struct folder_row swap = rows[i];
			rows[i] = rows[best];
			rows[best] = swap;
		}
	}

	gs_color(GS_COLOR_ACCENT);
	gs_printf("\nLargest folders in PSP/\n");
	gs_color(GS_COLOR_TEXT);
	if (row_count == 0) {
		gs_color(GS_COLOR_DIM);
		gs_printf("  (none of SAVEDATA, GAME, PICTURE, MUSIC, VIDEO, COMMON)\n");
	}
	for (i = 0; i < row_count; i++) {
		char size_text[32];
		gs_human_size(rows[i].bytes, size_text, sizeof(size_text));
		gs_printf("  %-10s %10s  %4d files\n", rows[i].name, size_text, rows[i].files);
	}

	walk(PSP_ROOT, 0, 0, &reclaim_files, &reclaim_bytes);
	gs_human_size(reclaim_bytes, reclaim_text, sizeof(reclaim_text));
	gs_color(GS_COLOR_WARN);
	gs_printf("\nReclaimable\n");
	gs_color(GS_COLOR_TEXT);
	gs_printf("  PSP/COMMON cache + 0-byte + *.tmp files\n");
	gs_printf("  %d files, %s\n", reclaim_files, reclaim_text);

	after_bytes = free_bytes + reclaim_bytes;
	gs_human_size(after_bytes, after_text, sizeof(after_text));
	gs_color(GS_COLOR_GOOD);
	gs_printf("\nFree after cleanup: %s\n", after_text);
	gs_color(GS_COLOR_DIM);
	gs_printf("SAVEDATA contents are never touched.\n");
	gs_color(GS_COLOR_TEXT);
	gs_footer("Cross: clean   Circle: exit");
	gs_present();
}

static void do_clean(void)
{
	u64 free_bytes = 0;
	u64 total_bytes = 0;
	u64 deleted_bytes = 0;
	int deleted_files = 0;
	char deleted_text[32];
	char free_text[32];

	gs_color(GS_COLOR_DIM);
	gs_printf("\nDeleting...\n");
	gs_present();
	walk(PSP_ROOT, 0, 1, &deleted_files, &deleted_bytes);
	gs_human_size(deleted_bytes, deleted_text, sizeof(deleted_text));

	gs_color(GS_COLOR_GOOD);
	gs_printf("Done: %d files removed, %s freed.\n", deleted_files, deleted_text);
	if (gs_ms0_space(&free_bytes, &total_bytes) == 0) {
		gs_human_size(free_bytes, free_text, sizeof(free_text));
		gs_color(GS_COLOR_TEXT);
		gs_printf("ms0: now has %s free.\n", free_text);
	}
	gs_color(GS_COLOR_DIM);
	gs_footer("Cross or Circle: exit");
	gs_present();
}

int main(void)
{
	int reclaim_files = 0;
	u64 reclaim_bytes = 0;

	gs_boot("GODsend Cleaner");
	gs_wait_release(0xFFFFFFFFu);

	if (!gs_is_dir(PSP_ROOT)) {
		gs_color(GS_COLOR_BAD);
		gs_printf("\nms0:/PSP was not found.\n");
		gs_color(GS_COLOR_DIM);
		gs_printf("Connect the stick and launch again.\n");
		gs_footer("Cross or Circle: exit");
		gs_present();
		gs_wait_confirm_cancel();
		gs_exit();
	}

	draw_report();
	walk(PSP_ROOT, 0, 0, &reclaim_files, &reclaim_bytes);

	if (reclaim_files == 0) {
		gs_footer("Nothing to clean. Circle: exit");
		gs_present();
		gs_wait_confirm_cancel();
		gs_exit();
	}

	if (!gs_confirm("Delete these leftovers now?")) {
		gs_color(GS_COLOR_DIM);
		gs_printf("Cancelled. Nothing was deleted.\n");
		gs_footer("Cross or Circle: exit");
		gs_present();
		gs_wait_confirm_cancel();
		gs_exit();
	}

	do_clean();
	gs_wait_confirm_cancel();
	gs_exit();
	return 0;
}
