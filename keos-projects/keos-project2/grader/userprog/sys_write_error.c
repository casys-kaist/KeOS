#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(int argc, char *argv[]) {
    char buf[24] = {0};
    int fd;

    ASSERT(write(-1, buf, 10) < 0);

    fd = open("hello", O_RDONLY);
    ASSERT(fd >= 3);

    ASSERT(write(fd, buf, 24) < 0);

    fd = open("hello", O_WRONLY);
    ASSERT(fd >= 3);

    ASSERT(write(fd, NULL, 10) < 0);

    printf("success ");
    return 0;
}
