TARGET = godsend-sysinfo
OBJS = main.o ../common/gs_ui.o ../common/gs_fs.o

CFLAGS = -O2 -G0 -Wall -I../common
CXXFLAGS = $(CFLAGS) -fno-exceptions -fno-rtti
ASFLAGS = $(CFLAGS)
LIBDIR =
LDFLAGS =
LIBS = -lpsppower -lpsputility -lpspumd -lpspwlan

BUILD_PRX = 1
EXTRA_TARGETS = EBOOT.PBP
PSP_EBOOT_TITLE = GODsend SysInfo

PSPSDK = $(shell psp-config --pspsdk-path)
include $(PSPSDK)/lib/build.mak
