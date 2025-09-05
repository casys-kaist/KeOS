#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(void) {
    int fd;
    char buf[24] = {0};

    fd = open("hello3", O_RDWR);
    ASSERT(fd >= 3);

    ASSERT(tell(fd) == 0);

    ASSERT(read(fd, buf, 7) == 7);
    ASSERT(memcmp(buf, "Welcome", 7) == 0);

    ASSERT(tell(fd) == 7);

    ASSERT(seek(fd, 0, SEEK_SET) == 0);
    ASSERT(tell(fd) == 0);

    ASSERT(write(fd, "Awesome", 7) == 7);
    ASSERT(tell(fd) == 7);

    ASSERT(seek(fd, 0, SEEK_END) == 140);
    ASSERT(tell(fd) == 140);

    printf("success ");
    return 0;
}
