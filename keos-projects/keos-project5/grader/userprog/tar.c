#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <fcntl.h>
#include <stat.h>
#include <syscall.h>
#include <stddef.h>
#include <mman.h>
#include <dirent.h>
#include <string.h>

#define loop while(1)

// Tar header structure (POSIX.1-1988 "ustar" format)
#define TAR_BLOCK_SIZE 512
struct tar_header {
    char name[100];     /* 0-99 */
    char mode[8];       /* 100-107 */
    char uid[8];        /* 108-115 */
    char gid[8];        /* 116-123 */
    char size[12];      /* 124-135 */
    char mtime[12];     /* 136-147 */
    char chksum[8];     /* 148-155 */
    char typeflag;      /* 156-156 */
    char linkname[100]; /* 157-256 */
    char magic[6];      /* 257-262 */
    char version[2];    /* 263-264 */
    char uname[32];     /* 265-296 */
    char gname[32];     /* 297-328 */
    char devmajor[8];   /* 329-336 */
    char devminor[8];   /* 337-344 */
    char prefix[155];   /* 345-500 */
    char pad[12];       /* 501-512 */
};

/**
 * Converts an octal string to a long integer.
 * This is a custom implementation to avoid using strtol from the standard library.
 *
 * @param octal_str The null-terminated string to convert.
 * @param size The maximum number of characters to read from the string.
 * @return The converted long integer.
 */
static long octal_to_long(const char *octal_str, int size) {
    long result = 0;
    int i = 0;
    while (i < size && octal_str[i] != '\0') {
        result = (result * 8) + (octal_str[i] - '0');
        i++;
    }
    return result;
}

/**
 * Calculates the checksum for a tar header.
 *
 * @param header Pointer to the tar header struct.
 * @return The calculated checksum.
 */
static long calculate_checksum(struct tar_header *header) {
    long sum = 0;
    char *p = (char *)header;
    int i;
    for (i = 0; i < TAR_BLOCK_SIZE; ++i) {
        if (i >= 148 && i < 156) { // Checksum field
            sum += ' ';
        } else {
            sum += p[i];
        }
    }
    return sum;
}

/**
 * Writes a tar header for a given file.
 *
 * @param fd File descriptor of the tar archive.
 * @param path Path to the file to archive.
 * @param st Pointer to the stat structure of the file.
 * @return 0 on success, -1 on failure.
 */
static int write_header(int fd, const char *path, struct stat *st) {
    struct tar_header header;
    memset(&header, 0, sizeof(header));

    // Fill header fields
    snprintf(header.name, sizeof(header.name), "%s", path);
    snprintf(header.mode, sizeof(header.mode), "%07o", (st->st_mode) & 0777);
    snprintf(header.uid, sizeof(header.uid), "%07o", st->st_uid);
    snprintf(header.gid, sizeof(header.gid), "%07o", st->st_gid);
    snprintf(header.size, sizeof(header.size), "%011o", (unsigned int)st->st_size);
    snprintf(header.mtime, sizeof(header.mtime), "%011o", (unsigned int)st->st_mtime);
    strlcpy(header.magic, "ustar", 4);
    strlcpy(header.version, "00", 2);

    // Determine file type
    if (S_ISREG(st->st_mode)) {
        header.typeflag = '0';
    } else if (S_ISDIR(st->st_mode)) {
        header.typeflag = '5';
        // Directories have a size of 0 in the tar header
        snprintf(header.size, sizeof(header.size), "%011o", 0);
    } else {
        printf("Skipping unsupported file type for %s\n", path);
        return 0;
    }

    // Calculate checksum and update header
    long chksum = calculate_checksum(&header);
    snprintf(header.chksum, sizeof(header.chksum), "%06o", (unsigned int)chksum);
    header.chksum[7] = ' ';

    int nwrite;
    if ((nwrite = write(fd, &header, sizeof(header))) != sizeof(header)) {
        printf("write header: errno %d\n", nwrite);
        return -1;
    }

    return 0;
}

/**
 * Recursively archives files and directories.
 *
 * @param tar_fd File descriptor of the tar archive.
 * @param path The path of the file/directory to archive.
 * @return 0 on success, -1 on failure.
 */
static int do_archive(int tar_fd, const char *path) {
    struct stat st;
    int stat_result;
    if ((stat_result = stat(path, &st)) < 0) {
        printf("stat: errno %d\n", stat_result);
        return -1;
    }

    printf("Archiving: %s\n", path);
    
    // Write header for the current file/directory
    if (write_header(tar_fd, path, &st) < 0) {
        return -1;
    }

    if (S_ISREG(st.st_mode)) {
        int file_fd = open(path, O_RDONLY);
        if (file_fd < 0) {
            printf("open: errno %d\n", file_fd);
            return -1;
        }

        // Use mmap to read file content without a large buffer
        char *file_data = mmap((void*)0xA000, st.st_size, PROT_READ, file_fd, 0);
        if (file_data < 0) {
            printf("mmap: errno %lld\n", (uint64_t)file_data);
            close(file_fd);
            return -1;
        }

        int write_bytes;
        if ((write_bytes = write(tar_fd, file_data, st.st_size)) != st.st_size) {
            printf("write file data: errno %d\n", write_bytes);
            munmap(file_data);
            close(file_fd);
            return -1;
        }

        // Pad to next 512-byte block
        int padding = (TAR_BLOCK_SIZE - (st.st_size % TAR_BLOCK_SIZE)) % TAR_BLOCK_SIZE;
        if (padding > 0) {
            char zeroes[TAR_BLOCK_SIZE] = {0};
            if ((write_bytes = write(tar_fd, zeroes, padding)) != padding) {
                printf("write padding: errno %d\n", write_bytes);
            }
        }

        munmap(file_data);
        close(file_fd);

    } else if (S_ISDIR(st.st_mode)) {
        // Open the directory and read its contents
        int dir_fd = open(path, O_RDONLY);
        if (dir_fd < 0) {
            printf("open directory: errno %d\n", dir_fd);
            return -1;
        }

        // Use mmap to allocate an anonymous memory region for getdents64
        // as a replacement for malloc.
        size_t dir_buf_size = TAR_BLOCK_SIZE * 2;
        uint64_t random_addr;
        getrandom(&random_addr, sizeof(uint64_t), 0);
        random_addr &= 0x00000FFFFFFFF000;
        char *dir_buf = mmap((void*)random_addr, dir_buf_size, PROT_READ | PROT_WRITE, -1, 0);
        if (dir_buf < 0) {
            printf("mmap: errno %lld\n", (uint64_t)dir_buf);
            close(dir_fd);
            return -1;
        }

        loop {
            long nread = readdir(dir_fd, (void*)dir_buf, dir_buf_size / 264) * 264;
            if (nread <= 0) break;

            long bpos;
            for (bpos = 0; bpos < nread;) {
                struct dirent *d = (struct dirent *)(dir_buf + bpos);

                // Skip . and .. entries
                if (strcmp(d->d_name, ".") != 0 && strcmp(d->d_name, "..") != 0) {
                    char full_path[276];
                    snprintf(full_path, sizeof(full_path), "%s/%s", path, d->d_name);
                    // Recursively archive the entry
                    if (do_archive(tar_fd, full_path) < 0) {
                        munmap(dir_buf);
                        close(dir_fd);
                        return -1;
                    }
                }
                bpos += d->d_reclen;
            }
        }
        munmap(dir_buf);
        close(dir_fd);
    }

    return 0;
}

/**
 * Extracts a tar archive.
 *
 * @param tar_fd File descriptor of the tar archive.
 * @return 0 on success, -1 on failure.
 */
static int do_extract(int tar_fd) {
    struct tar_header header;
    char buffer[TAR_BLOCK_SIZE];

    loop {
        int nread;
        if ((nread = read(tar_fd, &header, sizeof(header))) != sizeof(header)) {
            printf("read header: errno %d\n", nread);
            return -1;
        }

        // Check for end-of-file marker (two empty blocks)
        if (header.name[0] == 0) {
            break;
        }

        long file_size = octal_to_long(header.size, sizeof(header.size));
        long chksum = octal_to_long(header.chksum, sizeof(header.chksum));

        // Sanity check checksum
        long calculated_chksum = calculate_checksum(&header);
        if (chksum != calculated_chksum) {
            printf("Checksum mismatch for %s\n", header.name);
            continue;
        }

        printf("Extracting: %s (size: %ld)\n", header.name, file_size);

        if (header.typeflag == '5' || header.typeflag == 'L') { // Directory or symlink
            int mkdir_result;
            if ((mkdir_result = mkdir(header.name)) < 0) {
                printf("mkdir: errno %d\n", mkdir_result);
            }
            // Directories have size 0, no data to read
        } else if (header.typeflag == '0' || header.typeflag == 'L' || header.typeflag == '\0') { // Regular file
            int create_succeed = create(header.name);
            int file_fd;
            if (create_succeed < 0 || (file_fd = open(header.name, O_WRONLY)) < 0) {
                printf("open for write: errno %d\n", file_fd);
                seek(tar_fd, file_size, SEEK_CUR); // Skip file data
            } else {
                long bytes_left = file_size;
                while (bytes_left > 0) {
                    ssize_t to_read = (bytes_left > sizeof(buffer)) ? sizeof(buffer) : bytes_left;
                    ssize_t nread = read(tar_fd, buffer, to_read);
                    if (nread <= 0) {
                        printf("read file data: errno %lld\n", nread);
                        break;
                    }
                    int write_bytes;
                    if ((write_bytes = write(file_fd, buffer, nread)) != nread) {
                        printf("write file data: errno %d\n", write_bytes);
                        break;
                    }
                    bytes_left -= nread;
                }
                fsync(file_fd);
                close(file_fd);
            }
        } else {
            printf("Skipping unknown file type: %c for %s\n", header.typeflag, header.name);
            seek(tar_fd, file_size, SEEK_CUR); // Skip data block
        }
        
        // Skip padding
        int padding = (TAR_BLOCK_SIZE - (file_size % TAR_BLOCK_SIZE)) % TAR_BLOCK_SIZE;
        if (padding > 0) {
            seek(tar_fd, padding, SEEK_CUR);
        }
    }

    return 0;
}

/**
 * Main entry point of the tar-lite program.
 */
int main(int argc, char *argv[]) {
    if (argc < 3) {
        printf("Usage: %s -c <archive_file> <file1> [file2]...\n", argv[0]);
        printf("       %s -x <archive_file>\n", argv[0]);
        return 1;
    }

    const char *mode = argv[1];
    char *archive_path = argv[2];

    if (strcmp(mode, "-c") == 0) {
        int create_succeed = create(archive_path);
        int tar_fd;
        if (create_succeed < 0 || (tar_fd = open(archive_path, O_WRONLY)) < 0) {
            printf("open archive: errno %d\n", tar_fd);
            return 1;
        }

        int i;
        for (i = 3; i < argc; ++i) {
            if (do_archive(tar_fd, argv[i]) < 0) {
                close(tar_fd);
                return 1;
            }
        }
        
        // Write two empty blocks to mark the end of the archive
        char end_block[TAR_BLOCK_SIZE] = {0};
        write(tar_fd, end_block, sizeof(end_block));
        write(tar_fd, end_block, sizeof(end_block));

        fsync(tar_fd);
        close(tar_fd);
        printf("Archiving complete.\n");

    } else if (strcmp(mode, "-x") == 0) {
        int tar_fd = open(archive_path, O_RDONLY);
        if (tar_fd < 0) {
            printf("open archive: errno %d\n", tar_fd);
            return 1;
        }
        if (do_extract(tar_fd) < 0) {
            close(tar_fd);
            return 1;
        }
        close(tar_fd);
        printf("Extraction complete.\n");

    } else {
        printf("Unknown mode: %s\n", mode);
        return 1;
    }

    return 0;
}
