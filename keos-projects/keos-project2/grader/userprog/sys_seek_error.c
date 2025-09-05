#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(void) {
    int fd;

    ASSERT(seek(0, 0, SEEK_SET) < 0);

    ASSERT(seek(1, 0, SEEK_SET) < 0);

    ASSERT(seek(2, 0, SEEK_SET) < 0);

    ASSERT(seek(-1, 0, SEEK_SET) < 0);
    ASSERT(seek(-1, 0, SEEK_CUR) < 0);
    ASSERT(seek(-1, 0, SEEK_END) < 0);

    fd = open("hello", O_RDONLY);
    ASSERT(fd >= 3);

    ASSERT(seek(fd, 0, 3) < 0);

    printf("success ");
}
