#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(int argc, char *argv[]) {
    int fds[2] = {0};
    char buf[13] = {0};

    ASSERT(pipe(fds) == 0);
    ASSERT(fds[0] >= 3);
    ASSERT(fds[1] >= 4);
    ASSERT(fds[1] > fds[0]);

    ASSERT(read(fds[1], buf, 8) < 0);
    ASSERT(write(fds[0], buf, 8) < 0);

    ASSERT(write(fds[1], "Hello, keos!", 12) == 12);
    ASSERT(read(fds[0], buf, 12) == 12);

    ASSERT(strcmp(buf, "Hello, keos!") == 0);
    
    //ASSERT(close(fds[0]) == 0);
    //ASSERT(write(fds[1], "Hello, keos!", 12) < 0);

    printf("success ");
    return 0;
}
