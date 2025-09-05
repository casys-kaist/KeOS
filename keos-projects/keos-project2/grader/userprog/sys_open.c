#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>

int main(int argc, char *argv[]) {
    int fd1, fd2, fd3, fd4, fd5, fd6;
    
    fd1 = open("hello", O_RDONLY);
    ASSERT (fd1 >= 3);

    fd2 = open("hello", O_WRONLY);
    ASSERT (fd1 != fd2);

    fd3 = open("hello", O_RDWR);
    ASSERT (fd1 != fd3);
    ASSERT (fd2 != fd3);

    fd4 = open("nonexistant", O_RDONLY);
    ASSERT (fd4 < 0);

    fd5 = open(NULL, O_RDONLY);
    ASSERT (fd5 < 0);

    fd6 = open("hello", 9999);
    ASSERT (fd6 < 0);

    printf("success ");
    return 0;
}