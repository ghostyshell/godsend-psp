/*
 * godsend-saves - save folder backup utility for the PSP.
 *
 * Lists ms0:/PSP/SAVEDATA with per-folder sizes. D-pad up/down moves the
 * selection, Cross copies the selected folder to
 * ms0:/PSP/SAVEDATA/_backups/<name>-<timestamp> (plain recursive copy).
 */
#include <pspkernel.h>
#include <pspiofilemgr.h>
#include <pspdebug.h>

#include <stdio.h>
#include <string.h>
#include <time.h>

#include "gs_fs.h"
#include "gs_ui.h"

PSP_MODULE_INFO("GODsend Saves", 0, 1, 1);
PSP_MAIN_THREAD_ATTR(PSP_THREAD_ATTR_USER | PSP_THREAD_ATTR_VFPU);

#define SAVEDATA_DIR "ms0:/PSP/SAVEDATA"
#define BACKUP_DIR "ms0:/PSP/SAVEDATA/_backups"
#define MAX_SAVES 128
#define VISIBLE_ROWS 14

struct save_row {
	char name[80];
	u64 bytes;
	int files;
};

static struct save_row s_saves[MAX_SAVES];
static int s_save_count = 0;
static int s_selected = 0;
static int s_scroll = 0;

/* ARK_01234 is the CFW payload, not a save; _backups is our own output. */
static int is_skippable(const char *name)
{
	if (strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
		return 1;
	}
	if (strcmp(name, "_backups") == 0) {
		return 1;
	}
	if (strcmp(name, "ARK_01234") == 0) {
		return 1;
	}
	return 0;
}

static void load_saves(void)
{
	SceIoDirent entry;
	int fd;

	s_save_count = 0;
	s_selected = 0;
	s_scroll = 0;
	fd = sceIoDopen(SAVEDATA_DIR);
	if (fd < 0) {
		return;
	}
	for (;;) {
		char path[GS_PATH_MAX];
		memset(&entry, 0, sizeof(entry));
		if (sceIoDread(fd, &entry) <= 0) {
			break;
		}
		if (s_save_count >= MAX_SAVES) {
			break;
		}
		if (is_skippable(entry.d_name)) {
			continue;
		}
		if ((entry.d_stat.st_mode & FIO_S_IFMT) != FIO_S_IFDIR) {
			continue;
		}
		snprintf(path, sizeof(path), "%s/%s", SAVEDATA_DIR, entry.d_name);
		snprintf(s_saves[s_save_count].name, sizeof(s_saves[s_save_count].name), "%.79s", entry.d_name);
		s_saves[s_save_count].files = 0;
		s_saves[s_save_count].bytes = gs_dir_size(path, &s_saves[s_save_count].files);
		s_save_count++;
	}
	sceIoDclose(fd);
}

static void draw_list(const char *status, u32 status_color)
{
	u64 free_bytes = 0;
	u64 total_bytes = 0;
	char free_text[32];
	char total_text[32];
	int i;

	gs_reset("GODsend Saves");
	if (s_save_count == 0) {
		gs_color(GS_COLOR_WARN);
		gs_printf("\nNo save folders in ms0:/PSP/SAVEDATA.\n");
		gs_color(GS_COLOR_DIM);
		gs_printf("Play a game once, then come back.\n");
		gs_footer("Cross or Circle: exit");
		gs_present();
		return;
	}

	gs_printf("\n%d save folders. Cross backs up the selected one.\n\n", s_save_count);
	for (i = 0; i < VISIBLE_ROWS; i++) {
		int index = s_scroll + i;
		char size_text[32];
		if (index >= s_save_count) {
			break;
		}
		gs_human_size(s_saves[index].bytes, size_text, sizeof(size_text));
		if (index == s_selected) {
			pspDebugScreenSetBackColor(GS_COLOR_SELECT);
			gs_color(GS_COLOR_ACCENT);
			gs_printf(" > %-22.22s %10s\n", s_saves[index].name, size_text);
			pspDebugScreenSetBackColor(GS_COLOR_BLACK);
			gs_color(GS_COLOR_TEXT);
		} else {
			gs_printf("   %-22.22s %10s\n", s_saves[index].name, size_text);
		}
	}

	if (gs_ms0_space(&free_bytes, &total_bytes) == 0) {
		gs_human_size(free_bytes, free_text, sizeof(free_text));
		gs_human_size(total_bytes, total_text, sizeof(total_text));
		gs_color(GS_COLOR_DIM);
		gs_printf("\nms0: %s free of %s\n", free_text, total_text);
	}

	if (status != NULL) {
		gs_color(status_color);
		gs_printf("%s\n", status);
	}
	gs_color(GS_COLOR_DIM);
	gs_footer("Up/Down: select   Cross: back up   Circle: exit");
	gs_present();
}

static void scroll_to_selection(void)
{
	if (s_selected < s_scroll) {
		s_scroll = s_selected;
	}
	if (s_selected >= s_scroll + VISIBLE_ROWS) {
		s_scroll = s_selected - VISIBLE_ROWS + 1;
	}
	if (s_scroll < 0) {
		s_scroll = 0;
	}
}

static void backup_selected(void)
{
	char source[GS_PATH_MAX];
	char dest[GS_PATH_MAX];
	char stamp[24];
	char message[160];
	time_t now;
	struct tm local;
	int files = 0;
	u64 bytes = 0;
	char size_text[32];
	int result;

	now = time(NULL);
	memset(&local, 0, sizeof(local));
	{
		struct tm *clock = localtime(&now);
		if (clock != NULL) {
			local = *clock;
		}
	}
	strftime(stamp, sizeof(stamp), "%Y%m%d-%H%M%S", &local);

	snprintf(source, sizeof(source), "%s/%s", SAVEDATA_DIR, s_saves[s_selected].name);
	snprintf(dest, sizeof(dest), "%s/%s-%s", BACKUP_DIR, s_saves[s_selected].name, stamp);

	gs_color(GS_COLOR_WARN);
	gs_printf("\nBacking up %s ...\n", s_saves[s_selected].name);
	gs_present();

	if (gs_mkdir_p(BACKUP_DIR) < 0) {
		snprintf(message, sizeof(message), "FAILED: cannot create PSP/SAVEDATA/_backups");
		draw_list(message, GS_COLOR_BAD);
		return;
	}
	result = gs_copy_tree(source, dest, &files, &bytes);
	gs_human_size(bytes, size_text, sizeof(size_text));
	if (result == 0) {
		snprintf(message, sizeof(message), "OK: %d files, %s -> _backups/%s-%s",
			files, size_text, s_saves[s_selected].name, stamp);
		draw_list(message, GS_COLOR_GOOD);
	} else {
		snprintf(message, sizeof(message), "FAILED (0x%08X): %d files, %s copied",
			(unsigned int)result, files, size_text);
		draw_list(message, GS_COLOR_BAD);
	}
}

int main(void)
{
	u32 buttons;
	u32 last;
	const char *status = NULL;
	u32 status_color = GS_COLOR_TEXT;

	gs_boot("GODsend Saves");
	gs_wait_release(0xFFFFFFFFu);
	load_saves();
	draw_list(status, status_color);

	last = 0;
	for (;;) {
		buttons = gs_buttons();
		if ((buttons & (PSP_CTRL_UP | PSP_CTRL_DOWN)) != 0 && (last & (PSP_CTRL_UP | PSP_CTRL_DOWN)) == 0) {
			if (s_save_count > 0) {
				if (buttons & PSP_CTRL_UP) {
					s_selected = (s_selected + s_save_count - 1) % s_save_count;
				} else {
					s_selected = (s_selected + 1) % s_save_count;
				}
				scroll_to_selection();
				draw_list(status, status_color);
			}
		}
		if ((buttons & PSP_CTRL_CROSS) != 0 && (last & PSP_CTRL_CROSS) == 0) {
			gs_wait_release(PSP_CTRL_CROSS);
			if (s_save_count > 0) {
				backup_selected();
				status = NULL;
				status_color = GS_COLOR_TEXT;
			}
		}
		if ((buttons & PSP_CTRL_CIRCLE) != 0) {
			gs_exit();
		}
		if ((buttons & PSP_CTRL_HOME) != 0) {
			gs_exit();
		}
		last = buttons;
		gs_sleep(20);
	}
	return 0;
}
