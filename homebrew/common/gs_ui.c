#include "gs_ui.h"

#include <pspctrl.h>
#include <pspdebug.h>
#include <pspdisplay.h>
#include <pspkernel.h>

#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#define GS_ROWS 34

static u32 s_text = GS_COLOR_TEXT;

static void gs_set_colors(void)
{
	pspDebugScreenSetBackColor(GS_COLOR_BLACK);
	pspDebugScreenSetTextColor(GS_COLOR_TEXT);
}

void gs_boot(const char *title)
{
	pspDebugScreenInit();
	gs_set_colors();
	pspDebugScreenClear();
	sceCtrlSetSamplingCycle(0);
	sceCtrlSetSamplingMode(PSP_CTRL_MODE_ANALOG);
	gs_title(title);
	gs_present();
}

void gs_clear(void)
{
	pspDebugScreenClear();
	pspDebugScreenSetXY(0, 0);
	gs_color(s_text);
}

void gs_reset(const char *title)
{
	gs_clear();
	gs_title(title);
}

void gs_title(const char *title)
{
	pspDebugScreenSetXY(0, 0);
	pspDebugScreenSetTextColor(GS_COLOR_DIM);
	pspDebugScreenPrintf("GODsend PSP utilities\n");
	pspDebugScreenSetTextColor(GS_COLOR_ACCENT);
	pspDebugScreenPrintf("%s\n", title);
	gs_rule();
	s_text = GS_COLOR_TEXT;
	pspDebugScreenSetTextColor(s_text);
}

void gs_rule(void)
{
	pspDebugScreenSetTextColor(GS_COLOR_DIM);
	pspDebugScreenPrintf("------------------------------------------------------------\n");
	pspDebugScreenSetTextColor(s_text);
}

void gs_color(u32 color)
{
	s_text = color;
	pspDebugScreenSetTextColor(color);
}

void gs_printf(const char *fmt, ...)
{
	char line[512];
	va_list ap;
	va_start(ap, fmt);
	vsnprintf(line, sizeof(line), fmt, ap);
	va_end(ap);
	pspDebugScreenPrintf("%s", line);
}

void gs_footer(const char *text)
{
	int x = pspDebugScreenGetX();
	int y = pspDebugScreenGetY();
	pspDebugScreenSetXY(0, GS_ROWS - 2);
	pspDebugScreenSetBackColor(GS_COLOR_BLACK);
	pspDebugScreenSetTextColor(GS_COLOR_DIM);
	pspDebugScreenPrintf("%-60.60s", text);
	pspDebugScreenSetTextColor(s_text);
	pspDebugScreenSetXY(x, y);
}

u32 gs_buttons(void)
{
	SceCtrlData pad;
	memset(&pad, 0, sizeof(pad));
	if (sceCtrlReadBufferPositive(&pad, 1) < 0) {
		return 0;
	}
	return pad.Buttons;
}

void gs_wait_release(u32 mask)
{
	while ((gs_buttons() & mask) != 0) {
		sceKernelDelayThread(20000);
	}
}

void gs_sleep(int ms)
{
	if (ms > 0) {
		sceKernelDelayThread((SceUInt)(ms * 1000));
	}
}

void gs_present(void)
{
	sceDisplayWaitVblankStart();
}

u32 gs_wait_confirm_cancel(void)
{
	gs_wait_release(0xFFFFFFFFu);
	for (;;) {
		u32 buttons = gs_buttons();
		if (buttons & PSP_CTRL_CROSS) {
			gs_wait_release(PSP_CTRL_CROSS);
			return PSP_CTRL_CROSS;
		}
		if (buttons & PSP_CTRL_CIRCLE) {
			gs_wait_release(PSP_CTRL_CIRCLE);
			return PSP_CTRL_CIRCLE;
		}
		sceKernelDelayThread(20000);
	}
}

int gs_confirm(const char *question)
{
	gs_color(GS_COLOR_WARN);
	gs_printf("\n%s\n", question);
	gs_color(GS_COLOR_DIM);
	gs_printf("Cross = yes    Circle = no\n");
	gs_color(GS_COLOR_TEXT);
	return gs_wait_confirm_cancel() == PSP_CTRL_CROSS;
}

void gs_exit(void)
{
	sceKernelExitGame();
	sceKernelSleepThread();
}
