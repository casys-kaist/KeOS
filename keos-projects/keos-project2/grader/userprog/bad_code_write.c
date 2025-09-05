#include <syscall.h>
#include <debug.h>

int main(int argc, char *argv[]) {
    ASSERT(read(0, (void*)0x400000, 0x1000) < 0);
    ASSERT(pipe((void*)0x400000) < 0);

    *((int*)0x400000) = 0x42;

    return 0x1337; // This should NEVER executed
}