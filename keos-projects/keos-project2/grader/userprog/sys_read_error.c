#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(int argc, char *argv[]) {
    int fd1, fd2;
    char buf[24] = {0};

    ASSERT (read(-1, buf, 10) < 0);
    
    fd1 = open("hello", O_WRONLY);
    ASSERT (fd1 >= 3);

    ASSERT (read(fd1, buf, 24) < 0);

    fd2 = open("hello", O_RDONLY);
    ASSERT (fd2 >= 3);
    ASSERT (read(fd2, NULL, 10));

    printf("success ");
    return 0;
}