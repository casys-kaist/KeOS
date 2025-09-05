#include <stdio.h>
#include <syscall.h>
#include <fcntl.h>
#include <string.h>
#include <errno.h>
#include <mman.h>
#include <sha256.h> 

#define TRUELY_ERROR(x) ((int64_t)x < 0 && (-(int64_t)x) < 0x100)

int main(int argc, char *argv[]) {
    const char *source_name;
    BYTE hash[SHA256_BLOCK_SIZE];
    SHA256_CTX ctx;
    int fd;
    ssize_t bytes_read;
    char *buffer;

    sha256_init(&ctx);
    
    buffer = (char*)mmap((void*)0xA000UL, 0x1000, PROT_READ | PROT_WRITE, -1, 0);
    if (TRUELY_ERROR(buffer)) {
        printf("Error allocating memory: %lld\n", (uint64_t)buffer);
        return 1;
    }

    if (argc == 1) {
        source_name = "-"; 
        fd = STDIN_FILENO;
    } else if (argc == 2) {
        source_name = argv[1];
        
        fd = open(source_name, O_RDONLY);
        if (fd < 0) {
            printf("Error opening file %s: %d\n", source_name, fd);
            munmap(buffer);
            return 1;
        }
    } else {
        printf("Usage: %s [filename]\n", argv[0]);
        munmap(buffer);
        return 1;
    }

    while ((bytes_read = read(fd, buffer, 0x1000)) > 0) {
        sha256_update(&ctx, (const BYTE *)buffer, bytes_read);
    }

    if (bytes_read < 0) {
        printf("Error reading from %s: %lld\n", source_name, bytes_read);
        if (fd != STDIN_FILENO) {
            close(fd);
        }
        munmap(buffer);
        return 1;
    }

    if (fd != STDIN_FILENO) {
        close(fd);
    }
    
    munmap(buffer);
    sha256_final(&ctx, hash);

    for (int i = 0; i < SHA256_BLOCK_SIZE; i++) {
        printf("%02x", hash[i]);
    }
    printf("  %s\n", source_name);

    return 0;
}