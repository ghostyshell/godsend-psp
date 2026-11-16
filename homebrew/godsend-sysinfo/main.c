/*
 * godsend-sysinfo - read-only system report for the PSP.
 *
 * Firmware, model family, nickname, battery, memory stick space, MAC address
 * and UMD state. Nothing here writes to the stick; Cross redraws the report.
 */
#include <pspkernel.h>
#include <pspsysmem.h>
#include <psppower.h>
#include <psputility.h>
#include <psputility_sysparam.h>
#include <pspumd.h>
#include <pspwlan.h>

#include <stdio.h>
#include <string.h>

#include "gs_fs.h"
#include "gs_ui.h"

PSP_MODULE_INFO("GODsend SysInfo", 0, 1, 1);
PSP_MAIN_THREAD_ATTR(PSP_THREAD_ATTR_USER | PSP_THREAD_ATTR_VFPU);

static const char *model_label(void)
{
	/* User-mode code cannot query the 01g..09g model id (kernel only), so
	 * report what user mode can prove: the Go's internal ef0: storage. */
	if (gs_is_dir("ef0:/")) {
		return "PSP Go (N1000)";
	}
	return "PSP-1000/2000/3000 (user mode)";
}

static const char *battery_state(void)
{
	if (scePowerIsBatteryExist() <= 0) {
		return "no battery";
	}
	if (scePowerIsBatteryCharging() > 0) {
		return "charging";
	}
	if (scePowerIsPowerOnline() > 0) {
		return "on AC, not charging";
	}
	return "discharging";
}

static void read_mac(char *out, unsigned int len)
{
	unsigned char mac[8];
	int result;

	memset(mac, 0, sizeof(mac));
	result = sceWlanGetEtherAddr(mac);
	if (result < 0) {
		snprintf(out, len, "unavailable (0x%08X)", (unsigned int)result);
		return;
	}
	snprintf(out, len, "%02X:%02X:%02X:%02X:%02X:%02X",
		mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
}

static void draw_report(void)
{
	int version;
	unsigned int major;
	unsigned int minor;
	unsigned int build;
	char nickname[72];
	char mac[48];
	char free_text[32];
	char used_text[32];
	char total_text[32];
	u64 free_bytes = 0;
	u64 total_bytes = 0;
	u64 used_bytes = 0;
	int percent;
	int umd;
	int drive;

	gs_reset("GODsend SysInfo");

	version = sceKernelDevkitVersion();
	major = ((unsigned int)version >> 24) & 0xFF;
	minor = ((unsigned int)version >> 16) & 0xFF;
	build = (unsigned int)version & 0xFFFF;

	memset(nickname, 0, sizeof(nickname));
	if (sceUtilityGetSystemParamString(PSP_SYSTEMPARAM_ID_STRING_NICKNAME,
			nickname, sizeof(nickname)) < 0 || nickname[0] == '\0') {
		snprintf(nickname, sizeof(nickname), "(not set)");
	}

	gs_printf("\n");
	gs_color(GS_COLOR_DIM);
	gs_printf("Firmware    ");
	gs_color(GS_COLOR_TEXT);
	gs_printf("%u.%02u  (build %u)\n", major, minor, build);

	gs_color(GS_COLOR_DIM);
	gs_printf("Model       ");
	gs_color(GS_COLOR_TEXT);
	gs_printf("%s\n", model_label());

	gs_color(GS_COLOR_DIM);
	gs_printf("Nickname    ");
	gs_color(GS_COLOR_TEXT);
	gs_printf("%s\n", nickname);

	percent = scePowerGetBatteryLifePercent();
	gs_color(GS_COLOR_DIM);
	gs_printf("Battery     ");
	if (percent < 0) {
		gs_color(GS_COLOR_WARN);
		gs_printf("unknown\n");
	} else {
		gs_color(percent <= 15 ? GS_COLOR_BAD : GS_COLOR_TEXT);
		gs_printf("%d%%  %s\n", percent, battery_state());
	}

	if (gs_ms0_space(&free_bytes, &total_bytes) == 0) {
		used_bytes = total_bytes > free_bytes ? total_bytes - free_bytes : 0;
		gs_human_size(free_bytes, free_text, sizeof(free_text));
		gs_human_size(used_bytes, used_text, sizeof(used_text));
		gs_human_size(total_bytes, total_text, sizeof(total_text));
		gs_color(GS_COLOR_DIM);
		gs_printf("ms0:        ");
		gs_color(GS_COLOR_TEXT);
		gs_printf("%s free of %s\n", free_text, total_text);
		gs_color(GS_COLOR_DIM);
		gs_printf("            ");
		gs_color(GS_COLOR_TEXT);
		gs_printf("%s used\n", used_text);
	} else {
		gs_color(GS_COLOR_DIM);
		gs_printf("ms0:        ");
		gs_color(GS_COLOR_WARN);
		gs_printf("not mounted\n");
	}

	read_mac(mac, sizeof(mac));
	gs_color(GS_COLOR_DIM);
	gs_printf("MAC         ");
	gs_color(GS_COLOR_TEXT);
	gs_printf("%s\n", mac);

	umd = sceUmdCheckMedium();
	drive = sceUmdGetDriveStat();
	gs_color(GS_COLOR_DIM);
	gs_printf("UMD         ");
	if (umd > 0) {
		gs_color(GS_COLOR_GOOD);
		gs_printf("disc inserted (drive 0x%04X)\n", (unsigned int)drive & 0xFFFF);
	} else {
		gs_color(GS_COLOR_TEXT);
		gs_printf("no disc (drive 0x%04X)\n", (unsigned int)drive & 0xFFFF);
	}

	gs_color(GS_COLOR_DIM);
	gs_printf("\nRead only. Nothing is written to the stick.\n");
	gs_footer("Cross: refresh   Circle: exit");
	gs_present();
}

int main(void)
{
	u32 buttons;
	u32 last = 0;

	gs_boot("GODsend SysInfo");
	gs_wait_release(0xFFFFFFFFu);
	draw_report();

	for (;;) {
		buttons = gs_buttons();
		if ((buttons & PSP_CTRL_CROSS) != 0 && (last & PSP_CTRL_CROSS) == 0) {
			gs_wait_release(PSP_CTRL_CROSS);
			draw_report();
		}
		if ((buttons & PSP_CTRL_CIRCLE) != 0) {
			gs_exit();
		}
		last = buttons;
		gs_sleep(20);
	}
	return 0;
}
