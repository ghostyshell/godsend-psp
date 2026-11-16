/*
 * godsend-netcheck - WLAN and connection check for the PSP.
 *
 * Reports the WLAN switch, connects the configured access point through
 * sceNetApctlConnect, and prints the assigned IP, subnet, gateway and DNS
 * plus a clear pass/fail line. Cross re-runs the check.
 */
#include <pspkernel.h>
#include <pspnet.h>
#include <pspnet_apctl.h>
#include <pspwlan.h>

#include <stdio.h>
#include <string.h>

#include "gs_ui.h"

PSP_MODULE_INFO("GODsend NetCheck", 0, 1, 1);
PSP_MAIN_THREAD_ATTR(PSP_THREAD_ATTR_USER | PSP_THREAD_ATTR_VFPU);

#define NET_POOL_SIZE (128 * 1024)
#define APCTL_STACK_SIZE (32 * 1024)
#define CONNECT_TIMEOUT_MS 20000
#define POLL_MS 250
#define MAX_CONNECTIONS 5

static int s_net_ready = 0;
static int s_connection = 0;

static const char *state_label(int state)
{
	switch (state) {
	case PSP_NET_APCTL_STATE_DISCONNECTED:
		return "disconnected";
	case PSP_NET_APCTL_STATE_SCANNING:
		return "scanning";
	case PSP_NET_APCTL_STATE_JOINING:
		return "joining";
	case PSP_NET_APCTL_STATE_GETTING_IP:
		return "getting IP";
	case PSP_NET_APCTL_STATE_GOT_IP:
		return "got IP";
	case PSP_NET_APCTL_STATE_EAP_AUTH:
		return "EAP auth";
	case PSP_NET_APCTL_STATE_KEY_EXCHANGE:
		return "key exchange";
	default:
		return "unknown";
	}
}

static int net_start(void)
{
	int result;

	result = sceNetInit(NET_POOL_SIZE, 42, 0x4000, 42, 0x4000);
	if (result < 0) {
		return result;
	}
	result = sceNetApctlInit(APCTL_STACK_SIZE, 42);
	if (result < 0 && result != (int)0x80110601) { /* already initialised is fine */
		sceNetTerm();
		return result;
	}
	s_net_ready = 1;
	return 0;
}

static void net_stop(void)
{
	if (s_connection > 0) {
		sceNetApctlDisconnect();
		s_connection = 0;
	}
	if (s_net_ready) {
		sceNetTerm();
		s_net_ready = 0;
	}
}

struct check_result {
	int ok;
	int wlan_switch;
	int state;
	int connection;
	int error;
	char ip[32];
	char subnet[32];
	char gateway[32];
	char dns[64];
	char reason[96];
};

static void read_info(int which, char *out, unsigned int len)
{
	SceNetApctlInfo info;
	memset(&info, 0, sizeof(info));
	if (sceNetApctlGetInfo(which, &info) < 0) {
		snprintf(out, len, "-");
		return;
	}
	switch (which) {
	case PSP_NET_APCTL_INFO_IP:
		snprintf(out, len, "%s", info.ip[0] ? info.ip : "-");
		break;
	case PSP_NET_APCTL_INFO_SUBNETMASK:
		snprintf(out, len, "%s", info.subNetMask[0] ? info.subNetMask : "-");
		break;
	case PSP_NET_APCTL_INFO_GATEWAY:
		snprintf(out, len, "%s", info.gateway[0] ? info.gateway : "-");
		break;
	case PSP_NET_APCTL_INFO_PRIMDNS: {
		SceNetApctlInfo secondary;
		memset(&secondary, 0, sizeof(secondary));
		sceNetApctlGetInfo(PSP_NET_APCTL_INFO_SECDNS, &secondary);
		snprintf(out, len, "%s / %s",
			info.primaryDns[0] ? info.primaryDns : "-",
			secondary.secondaryDns[0] ? secondary.secondaryDns : "-");
		break;
	}
	default:
		snprintf(out, len, "-");
		break;
	}
}

static void run_check(struct check_result *result)
{
	int index;
	int waited;
	int started;

	memset(result, 0, sizeof(*result));
	result->wlan_switch = sceWlanGetSwitchState();
	result->connection = -1;
	if (result->wlan_switch <= 0) {
		snprintf(result->reason, sizeof(result->reason), "WLAN switch is off");
		return;
	}

	if (!s_net_ready) {
		result->error = net_start();
		if (result->error < 0) {
			snprintf(result->reason, sizeof(result->reason),
				"network stack failed to start (0x%08X)", (unsigned int)result->error);
			return;
		}
	}

	started = 0;
	for (index = 1; index <= MAX_CONNECTIONS && !started; index++) {
		result->error = sceNetApctlConnect(index);
		if (result->error >= 0) {
			started = 1;
			s_connection = index;
			result->connection = index;
			break;
		}
	}
	if (!started) {
		snprintf(result->reason, sizeof(result->reason),
			"no configured connection answered (0x%08X)", (unsigned int)result->error);
		return;
	}

	waited = 0;
	for (;;) {
		int state = PSP_NET_APCTL_STATE_DISCONNECTED;
		if (sceNetApctlGetState(&state) >= 0) {
			result->state = state;
			if (state == PSP_NET_APCTL_STATE_GOT_IP) {
				break;
			}
			if (state == PSP_NET_APCTL_STATE_DISCONNECTED && waited > 2000) {
				snprintf(result->reason, sizeof(result->reason),
					"connection dropped while associating");
				return;
			}
		}
		if (waited >= CONNECT_TIMEOUT_MS) {
			snprintf(result->reason, sizeof(result->reason),
				"no IP after %d seconds (state: %s)", CONNECT_TIMEOUT_MS / 1000,
				state_label(result->state));
			return;
		}
		gs_sleep(POLL_MS);
		waited += POLL_MS;
	}

	read_info(PSP_NET_APCTL_INFO_IP, result->ip, sizeof(result->ip));
	read_info(PSP_NET_APCTL_INFO_SUBNETMASK, result->subnet, sizeof(result->subnet));
	read_info(PSP_NET_APCTL_INFO_GATEWAY, result->gateway, sizeof(result->gateway));
	read_info(PSP_NET_APCTL_INFO_PRIMDNS, result->dns, sizeof(result->dns));

	if (result->ip[0] != '\0' && strcmp(result->ip, "-") != 0) {
		result->ok = 1;
	} else {
		snprintf(result->reason, sizeof(result->reason), "connected but no IP was reported");
	}
}

static void draw_screen(const char *phase, const struct check_result *result)
{
	gs_reset("GODsend NetCheck");

	gs_color(GS_COLOR_DIM);
	gs_printf("\nWLAN switch   ");
	if (result->wlan_switch > 0) {
		gs_color(GS_COLOR_GOOD);
		gs_printf("on\n");
	} else {
		gs_color(GS_COLOR_BAD);
		gs_printf("off\n");
	}

	gs_color(GS_COLOR_DIM);
	gs_printf("Connection    ");
	gs_color(GS_COLOR_TEXT);
	if (result->connection > 0) {
		gs_printf("%d\n", result->connection);
	} else {
		gs_printf("-\n");
	}

	gs_color(GS_COLOR_DIM);
	gs_printf("State         ");
	gs_color(result->state == PSP_NET_APCTL_STATE_GOT_IP ? GS_COLOR_GOOD : GS_COLOR_TEXT);
	gs_printf("%s\n", state_label(result->state));

	gs_color(GS_COLOR_DIM);
	gs_printf("IP            ");
	gs_color(GS_COLOR_TEXT);
	gs_printf("%s\n", result->ip[0] ? result->ip : "-");
	gs_color(GS_COLOR_DIM);
	gs_printf("Subnet        ");
	gs_color(GS_COLOR_TEXT);
	gs_printf("%s\n", result->subnet[0] ? result->subnet : "-");
	gs_color(GS_COLOR_DIM);
	gs_printf("Gateway       ");
	gs_color(GS_COLOR_TEXT);
	gs_printf("%s\n", result->gateway[0] ? result->gateway : "-");
	gs_color(GS_COLOR_DIM);
	gs_printf("DNS           ");
	gs_color(GS_COLOR_TEXT);
	gs_printf("%s\n", result->dns[0] ? result->dns : "-");

	gs_printf("\n");
	if (phase != NULL) {
		gs_color(GS_COLOR_WARN);
		gs_printf("%s\n", phase);
	} else if (result->ok) {
		gs_color(GS_COLOR_GOOD);
		gs_printf("PASS: connection %d has %s\n", result->connection, result->ip);
	} else {
		gs_color(GS_COLOR_BAD);
		gs_printf("FAIL: %s\n", result->reason[0] ? result->reason : "not connected");
	}
	gs_color(GS_COLOR_DIM);
	gs_printf("\nPass means the PSP holds an IP on the configured\n");
	gs_printf("access point. This app does not move any data.\n");
	gs_footer("Cross: check again   Circle: exit");
	gs_present();
}

int main(void)
{
	struct check_result result;
	u32 buttons;
	u32 last = 0;

	gs_boot("GODsend NetCheck");
	gs_wait_release(0xFFFFFFFFu);

	memset(&result, 0, sizeof(result));
	result.wlan_switch = sceWlanGetSwitchState();
	draw_screen("Connecting to the configured access point...", &result);
	run_check(&result);
	draw_screen(NULL, &result);

	for (;;) {
		buttons = gs_buttons();
		if ((buttons & PSP_CTRL_CROSS) != 0 && (last & PSP_CTRL_CROSS) == 0) {
			gs_wait_release(PSP_CTRL_CROSS);
			net_stop();
			memset(&result, 0, sizeof(result));
			result.wlan_switch = sceWlanGetSwitchState();
			draw_screen("Connecting to the configured access point...", &result);
			run_check(&result);
			draw_screen(NULL, &result);
		}
		if ((buttons & PSP_CTRL_CIRCLE) != 0) {
			net_stop();
			gs_exit();
		}
		last = buttons;
		gs_sleep(20);
	}
	return 0;
}
