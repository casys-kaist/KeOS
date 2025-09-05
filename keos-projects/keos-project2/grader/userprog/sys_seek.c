#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(void) {
    int fd;
    char buf[24] = {0};
    off_t new_offset;

    fd = open("hello", O_RDONLY);
    ASSERT(fd >= 3);

    new_offset = seek(fd, 0, SEEK_SET);
    ASSERT(new_offset == 0);

    ssize_t bytes_read = read(fd, buf, 24);
    ASSERT(bytes_read == 24);
    ASSERT(memcmp(buf, "Welcome to KeOS Project!", 24) == 0);

    new_offset = seek(fd, 0, SEEK_CUR);
    ASSERT(new_offset == 24);

    new_offset = seek(fd, -13, SEEK_CUR);
    ASSERT(new_offset == 11);

    bytes_read = read(fd, buf, 4);
    ASSERT(bytes_read == 4);
    ASSERT(memcmp(buf, "KeOS", 4) == 0);

    ASSERT(close(fd) == 0);
    printf("success ");
    return 0;
}
