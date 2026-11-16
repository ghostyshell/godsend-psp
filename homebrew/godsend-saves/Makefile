TARGET = godsend-saves
OBJS = main.o ../common/gs_ui.o ../common/gs_fs.o

CFLAGS = -O2 -G0 -Wall -I../common
CXXFLAGS = $(CFLAGS) -fno-exceptions -fno-rtti
ASFLAGS = $(CFLAGS)
LIBDIR =
LDFLAGS =
LIBS =

BUILD_PRX = 1
EXTRA_TARGETS = EBOOT.PBP
PSP_EBOOT_ICON = icon0.png
PSP_EBOOT_PIC1 = pic1.png
PSP_EBOOT_TITLE = GODsend Saves

PSPSDK = $(shell psp-config --pspsdk-path)
include $(PSPSDK)/lib/build.mak
