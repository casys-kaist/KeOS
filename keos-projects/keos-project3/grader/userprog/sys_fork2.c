#include <debug.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>
#include <mman.h>

int value = 0;

int main(int argc, char *argv[]) {
    int pid;
    int fds[2] = {0};
    char buf[1] = {0};

    ASSERT(pipe(fds) == 0);
    ASSERT(fds[0] >= 3);
    ASSERT(fds[1] >= 4);
    ASSERT(fds[1] > fds[0]);

    // create a read-only anonymous mapping
    ASSERT(mmap((void *)0xA000, 0x1000, PROT_READ, -1, 0) == 0xA000);

    // create a read-only file-backed mapping
    int fd = open("hello", O_RDONLY);
    ASSERT(mmap((void *)0xB000, 0x1000, PROT_READ, fd, 0) == 0xB000);

    // read-write file backed mapping
    int fd2 = open("hello2", O_RDWR);
    ASSERT(mmap((void *)0xD000, 0x1000, PROT_READ | PROT_WRITE, fd2, 0) == 0xD000);

    pid = fork();
    ASSERT(pid == 0 || pid > 0);
    if (pid == 0) {
        printf("Hello, parent!\n");

        char* data = (char *) 0xD000;
        data[2] = ' ';
        ASSERT(memcmp((char *)0xD000, "We come to KeOS Project!", 24) == 0);
        printf("Child edited successfully!\n");

        char* data2 = (char *) 0xB000;
        ASSERT(write(fds[1], "\0", 1) == 1);
        data2[3] = '@';
        printf("Child edited again!\n");
    } else {
        ASSERT(read(fds[0], buf, 1) == 1);
        printf("Hello, child!\n");
        ASSERT(memcmp((char *)0xD000, "Welcome to KeOS Project!", 24) == 0);

        (*((int *)0xA000))++;
        return 21;
    }

    return 0;
}