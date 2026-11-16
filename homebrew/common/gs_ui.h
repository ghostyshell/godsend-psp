/*
 * gs_ui.h - tiny text UI helpers shared by the GODsend PSP utilities.
 *
 * Every app is a plain user-mode EBOOT that draws with PSPSDK's debug screen
 * (8x8 font, 60x34 character cells on a 480x272 display) and reads the d-pad
 * and face buttons with sceCtrl.
 */
#ifndef GS_UI_H
#define GS_UI_H

#include <psptypes.h>
#include <pspctrl.h>

#define GS_COLOR_TEXT 0xFFFFFFFFu
#define GS_COLOR_DIM 0xFF8A9099u
#define GS_COLOR_ACCENT 0xFF4DD8FFu
#define GS_COLOR_GOOD 0xFF5FE08Cu
#define GS_COLOR_WARN 0xFFFFC24Du
#define GS_COLOR_BAD 0xFFFF6B6Bu
#define GS_COLOR_SELECT 0xFF153A55u
#define GS_COLOR_BLACK 0xFF000000u

/* Boot the debug screen, sample the pad at 60 Hz, draw the banner. */
void gs_boot(const char *title);
/* Wipe the character grid and draw the banner again. */
void gs_reset(const char *title);
void gs_clear(void);
void gs_title(const char *title);

void gs_color(u32 color);
void gs_printf(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
/* A full-width dim rule. */
void gs_rule(void);
/* Bottom status line at row 32. */
void gs_footer(const char *text);

/* Current button state, one non-blocking sample. */
u32 gs_buttons(void);
/* Wait until every button is released (call right after acting on a press). */
void gs_wait_release(u32 mask);
void gs_sleep(int ms);
/* Flush the character grid to the display. */
void gs_present(void);

/* Yes/no prompt: Cross accepts, Circle cancels. Returns 1 or 0. */
int gs_confirm(const char *question);
/* Wait for Cross or Circle and return the button mask that was pressed. */
u32 gs_wait_confirm_cancel(void);

void gs_exit(void);

#endif
