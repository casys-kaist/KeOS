#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

int main(void) {
    ASSERT(tell(0) < 0);
    ASSERT(tell(1) < 0);
    ASSERT(tell(2) < 0);
    ASSERT(tell(3123) < 0);

    printf("success ");
    return 0;
}
