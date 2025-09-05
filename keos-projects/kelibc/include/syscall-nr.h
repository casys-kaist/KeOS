#ifndef __LIB_SYSCALLSYS_H
#define __LIB_SYSCALLSYS_H

#define SYS_EXIT 0
#define SYS_OPEN 1
#define SYS_READ 2
#define SYS_WRITE 3
#define SYS_SEEK 4
#define SYS_TELL 5
#define SYS_CLOSE 6
#define SYS_PIPE 7
#define SYS_MMAP 8
#define SYS_MUNMAP 9
#define SYS_FORK 10
#define SYS_THREAD_CREATE 11
#define SYS_THREAD_JOIN 12
#define SYS_EXIT_GROUP 13
#define SYS_CREATE 14
#define SYS_MKDIR 15
#define SYS_UNLINK 16
#define SYS_CHDIR 17
#define SYS_READDIR 18
#define SYS_STAT 19
#define SYS_FSYNC 20

/* Only used for Project 3 CoW grading */
#define SYS_GETPHYS 0x81

#endif /* lib/syscall-nr.h */