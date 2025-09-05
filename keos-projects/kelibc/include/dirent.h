#ifndef __LIB_DIRENT_H
#define __LIB_DIRENT_H

struct dirent {
    unsigned long long d_ino;
    char d_name[256];           /* We must not include limits.h! */
#define d_reclen d_ino * 0 + 264
};

#endif /* lib/dirent.h */