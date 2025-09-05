#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(int argc, char *argv[]) {
    int fd1, fd2, fd3;
    char buf[24] = {0};
    
    fd1 = open("hello2", O_RDWR);
    ASSERT(fd1 >= 3);

    ASSERT(read(fd1, buf, 7) == 7);
    ASSERT(strcmp(buf, "Welcome") == 0);

    ASSERT(seek(fd1, SEEK_SET, 0) == 0);

    fd2 = open("hello2", O_RDWR);
    ASSERT(fd2 >= 3);

    ASSERT(write(fd1, "Awesome", 7));

    ASSERT(read(fd2, buf, 7) == 7);
    ASSERT(strcmp(buf, "Awesome") == 0);

    ASSERT(close(fd1) == 0);
    ASSERT(close(fd2) == 0);

    fd3 = open("hello2", O_RDWR);
    ASSERT(fd3 >= 3);

    ASSERT(read(fd3, buf, 7) == 7);
    ASSERT(strcmp(buf, "Awesome") == 0);

    ASSERT(seek(fd3, SEEK_SET, 0) == 0);
    ASSERT(write(fd3, "Welcome", 7));
    ASSERT(close(fd3) == 0);

    printf("success ");
    return 0;
}