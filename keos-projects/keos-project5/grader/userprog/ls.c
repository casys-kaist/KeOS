#include <stdio.h>
#include <syscall.h>
#include <mman.h>
#include <stat.h>
#include <string.h>
#include <fcntl.h>

// Function to print file permissions string
void print_permissions(mode_t mode) {
    // Determine file type
    if (S_ISDIR(mode)) {
        printf("d");
    } else if (S_ISLNK(mode)) {
        printf("l");
    } else if (S_ISCHR(mode)) {
        printf("c");
    } else if (S_ISBLK(mode)) {
        printf("b");
    } else if (S_ISFIFO(mode)) {
        printf("p");
    } else if (S_ISSOCK(mode)) {
        printf("s");
    } else {
        printf("-");
    }

    // Print permissions for owner, group, and others
    printf((mode & S_IRUSR) ? "r" : "-");
    printf((mode & S_IWUSR) ? "w" : "-");
    printf((mode & S_IXUSR) ? "x" : "-");
    printf((mode & S_IRGRP) ? "r" : "-");
    printf((mode & S_IWGRP) ? "w" : "-");
    printf((mode & S_IXGRP) ? "x" : "-");
    printf((mode & S_IROTH) ? "r" : "-");
    printf((mode & S_IWOTH) ? "w" : "-");
    printf((mode & S_IXOTH) ? "x" : "-");
}

// Function to format and print the modification time manually
// This is a simplified version of what would be provided by a time library.
void print_time_manual(long long mod_time) {
    const char *months[] = {"Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"};
    long long seconds_in_day = 24LL * 3600LL;
    long long days = mod_time / seconds_in_day;
    long long seconds_left = mod_time % seconds_in_day;

    int hour = seconds_left / 3600;
    int minute = (seconds_left % 3600) / 60;
    
    // A simplified approach to calculate date from days since epoch
    long long year = 1970;
    int month = 0;
    int day_of_month = 0;
    
    // Naively calculate year, month, and day. This doesn't account for all time zone issues
    // or leap years perfectly for all dates, but it's a simple, self-contained solution.
    long long days_in_year;
    
    while(days >= 365) {
        days_in_year = (year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)) ? 366 : 365;
        if (days < days_in_year) break;
        days -= days_in_year;
        year++;
    }

    int days_in_month[] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
    if ((year % 4 == 0 && year % 100 != 0) || year % 400 == 0) {
        days_in_month[1] = 29;
    }

    while (days >= days_in_month[month]) {
        days -= days_in_month[month];
        month++;
    }

    day_of_month = days + 1;

    printf("%s %2d %02d:%02d ", months[month], day_of_month, hour, minute);
}

// Main function to execute the program
int main(int argc, char *argv[]) {
    int long_format = 0;
    int show_all = 0;
    const char *dir_path = ".";

    // Parse command line arguments
    for (int i = 1; i < argc; i++) {
        if (argv[i][0] == '-') {
            // It's a flag
            for (int j = 1; argv[i][j] != '\0'; j++) {
                if (argv[i][j] == 'l') {
                    long_format = 1;
                } else if (argv[i][j] == 'a') {
                    show_all = 1;
                } else {
                    printf("ls: invalid option -- '%c'\n", argv[i][j]);
                    return 1;
                }
            }
        } else {
            // It's a directory path
            dir_path = argv[i];
        }
    }

    // Open the directory
    int dir_fd = open(dir_path, O_RDONLY);
    if (dir_fd < 0) {
        printf("ls: cannot access '%s': errno %d\n", dir_path, dir_fd);
        return 1;
    }

    // Buffer for directory entries
    char* buffer = mmap((void*)0xA000, 0x1000, PROT_READ | PROT_WRITE, -1, 0);
    if (buffer < 0) {
        printf("ls: failed to allocate memory with mmap\n");
        close(dir_fd);
        return 1;
    }
    ssize_t nread;
    unsigned long total_blocks = 0;

    // A list to store filenames for later processing
    const int MAX_FILES = 1024;
    const int MAX_FILENAME_LEN = 256;
    char *filenames = mmap((void*)0xB000, MAX_FILES * MAX_FILENAME_LEN, PROT_READ | PROT_WRITE, -1, 0);
    if (filenames < 0) {
        printf("ls: failed to allocate memory with mmap\n");
        munmap(buffer);
        close(dir_fd);
        return 1;
    }
    int file_count = 0;

    // Read directory entries using getdents64()
    while ((nread = readdir(dir_fd, (void*)buffer, 15)) > 0) {
        for (ssize_t bpos = 0; bpos < (264 * nread);) {
            struct dirent *d = (struct dirent *)(buffer + bpos);
            
            // Skip hidden files if -a is not set
            if (!show_all && d->d_name[0] == '.') {
                bpos += d->d_reclen;
                continue;
            }

            // Construct the full path to get file stats
            char full_path[1024];
            snprintf(full_path, sizeof(full_path), "%s/%s", dir_path, d->d_name);

            struct stat st;
            int errno;
            if ((errno = stat(full_path, &st)) < 0) {
                printf("ls: cannot access '%s': errno %d\n", full_path, errno);
                bpos += d->d_reclen;
                continue;
            }

            // Store filename and accumulate total blocks if in long format
            if (long_format) {
                total_blocks += (st.st_size + 4095) / 4096 * 4;
            }
            if (file_count < MAX_FILES) {
                strlcpy(&filenames[file_count * MAX_FILENAME_LEN], d->d_name, MAX_FILENAME_LEN);
                file_count++;
            } else {
                printf("ls: too many files to display\n");
                break;
            }

            bpos += d->d_reclen;
        }
    }

    // Check for errors on syscall
    if (nread < 0) {
        printf("ls: error reading directory entries (errno %lld)\n", nread);
        munmap(filenames);
        close(dir_fd);
        return 1;
    }
    
    // Print total block count if in long format
    if (long_format) {
        printf("total %lu\n", total_blocks);
    }
    
    // Sort filenames alphabetically
    for (int i = 0; i < file_count - 1; i++) {
        for (int j = i + 1; j < file_count; j++) {
            if (strcmp(&filenames[i * MAX_FILENAME_LEN], &filenames[j * MAX_FILENAME_LEN]) > 0) {
                char temp[MAX_FILENAME_LEN];
                strlcpy(temp, &filenames[i * MAX_FILENAME_LEN], MAX_FILENAME_LEN);
                strlcpy(&filenames[i * MAX_FILENAME_LEN], &filenames[j * MAX_FILENAME_LEN], MAX_FILENAME_LEN);
                strlcpy(&filenames[j * MAX_FILENAME_LEN], temp, MAX_FILENAME_LEN);
            }
        }
    }

    // Now, iterate through the sorted filenames to print the details
    for (int i = 0; i < file_count; i++) {
        char full_path[1024];
        snprintf(full_path, sizeof(full_path), "%s/%s", dir_path, &filenames[i * MAX_FILENAME_LEN]);
        
        struct stat st;
        int errno;
        if ((errno = stat((char*)full_path, &st)) < 0) {
            printf("ls: cannot access '%s': errno %d\n", full_path, errno);
            continue;
        }

        if (long_format) {
            print_permissions(st.st_mode);
            printf("    ");
            printf("%-4d %-4d ", st.st_uid, st.st_gid);
            printf("%8lld ", st.st_size);
            print_time_manual(st.st_mtime);
        }
        printf("%s\n", &filenames[i * MAX_FILENAME_LEN]);
    }

    close(dir_fd);
    munmap(buffer);
    munmap(filenames);
    return 0;
}
