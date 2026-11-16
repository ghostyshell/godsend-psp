TARGET = godsend-netcheck
OBJS = main.o ../common/gs_ui.o

CFLAGS = -O2 -G0 -Wall -I../common
CXXFLAGS = $(CFLAGS) -fno-exceptions -fno-rtti
ASFLAGS = $(CFLAGS)
LIBDIR =
LDFLAGS =
LIBS = -lpspwlan -lpspnet_inet

BUILD_PRX = 1
EXTRA_TARGETS = EBOOT.PBP
PSP_EBOOT_TITLE = GODsend NetCheck

PSPSDK = $(shell psp-config --pspsdk-path)
include $(PSPSDK)/lib/build.mak
