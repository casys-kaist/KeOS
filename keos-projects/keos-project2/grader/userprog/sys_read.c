#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(int argc, char *argv[]) {
    int fd;
    char buf[180] = {0};
    
    fd = open("hello", O_RDONLY);
    ASSERT (fd >= 3);

    ASSERT (read(fd, buf, 24) == 24);
    ASSERT (strcmp(buf, "Welcome to KeOS Project!") == 0);
    
    ASSERT (read(fd, buf, 8) == 8);
    ASSERT (strcmp(buf, "\n\nEven tto KeOS Project!") == 0);

    ASSERT (read(fd, buf, 0) == 0);
    ASSERT (strcmp(buf, "\n\nEven tto KeOS Project!") == 0);

    ASSERT (read(fd, buf, 180) == 108);
    ASSERT (read(fd, buf, 180) == 0);

    printf("success ");
    return 0;
}