CC = gcc
LD = ld
AR = ar
CFLAGS = -Wall -c -fno-stack-protector -Wno-builtin-declaration-mismatch -I../../../kelibc/include -mno-sse -g
LD_SCRIPT ?= ../../../kelibc/user.lds
DEFINES ?=
LDFLAGS = -nostdlib -static -T ${LD_SCRIPT}

# Use project-specific build directory for objects
BUILD_DIR ?= build_objects
OBJS  = $(addprefix $(BUILD_DIR)/,$(PROGS:=.o))

# Source files with project-specific object output
LIB_SOURCES = ../../../kelibc/string.c ../../../kelibc/syscall.c ../../../kelibc/console.c ../../../kelibc/entry.c ../../../kelibc/stdio.c ../../../kelibc/stdlib.c ../../../kelibc/debug.c ../../../kelibc/arithmetic.c
LIB_OBJECTS = $(addprefix $(BUILD_DIR)/,$(notdir $(LIB_SOURCES:.c=.o)))
LIB_NAME = $(BUILD_DIR)/kelibc.a

all: dir clean_entry $(LIB_NAME) $(PROGS) install

dir:
	mkdir -p build
	mkdir -p $(BUILD_DIR)

clean_entry:
	rm -f $(BUILD_DIR)/entry.o

install:
	cp build/* ../rootfs

$(LIB_NAME): $(LIB_OBJECTS)
	$(AR) rcs $@ $^

# Rule to compile library sources into BUILD_DIR
$(BUILD_DIR)/%.o: ../../../kelibc/%.c
	$(CC) $(CFLAGS) $(DEFINES) -o $@ $<

# Rule to compile main program sources into BUILD_DIR
$(BUILD_DIR)/%.o: %.c
	$(CC) $(CFLAGS) $(DEFINES) -o $@ $<

# Rule to link programs - output executables to build/ directory
$(PROGS): %: $(BUILD_DIR)/%.o $(LIB_NAME)
	$(LD) $(LDFLAGS) -o build/$@ $^

clean:
	rm -rf $(BUILD_DIR) build
	rm -f $(PROGS) $(OBJS)