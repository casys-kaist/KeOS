#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(int argc, char *argv[]) {
    char buf[24] = {0};
    int fd;

    ASSERT ((fd = open("hello", O_RDWR)) >= 3);

    ASSERT (close(fd) == 0);
    ASSERT (close(fd) < 0);
    ASSERT (write(fd, buf, 24) < 0);

    ASSERT (close(9222) < 0);

    ASSERT (read(0, buf, 7) == 7);
    ASSERT (close(0) == 0);
    ASSERT (read(0, buf, 7) < 0);

    ASSERT (close(1) == 0);
    ASSERT (write(1, buf, 7) < 0);

    ASSERT (write(2, buf, 7) < 0);
    ASSERT (close(2) == 0);
    ASSERT (write(2, buf, 7) < 0);

    printf("success ");
    return 0;
}