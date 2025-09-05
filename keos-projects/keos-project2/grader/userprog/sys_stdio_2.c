#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(int argc, char *argv[]) {
    char buf[8] = {0};
    ASSERT (read(0, buf, 5) == 5);
    ASSERT (strcmp(buf, "Hello") == 0);
    
    ASSERT (read(0, buf, 7) == 7);
    ASSERT (strcmp(buf, ", World") == 0);

    ASSERT (read(0, buf, 4) == 0);

    printf("success ");
    return 0;
}