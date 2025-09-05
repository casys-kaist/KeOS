#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(int argc, char *argv[]) {
    char buf[12] = {0};
    ASSERT (read(2, buf, 12) < 0);

    ASSERT (write(2, "Hello, keos!", 12) == 12);

    ASSERT (write(2, "", 0) == 0);

    ASSERT (write(2, NULL, 12) < 0);

    printf("success ");
    return 0;
}