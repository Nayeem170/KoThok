#include <fcntl.h>
#include <stdio.h>
#include <errno.h>
#include <unistd.h>
#include <linux/input.h>
#include <sys/ioctl.h>

int main(void) {
    int fd = open("/dev/input/event2", O_RDWR);
    if (fd < 0) { perror("open event2"); return 1; }
    if (ioctl(fd, EVIOCGRAB, (int)1) < 0) {
        if (errno == EBUSY)
            printf("EXCLUSIVE GRAB CONFIRMED\n");
        else
            printf("error: %s\n", strerror(errno));
    } else {
        ioctl(fd, EVIOCGRAB, (int)0);
        printf("NO EXCLUSIVE GRAB\n");
    }
    close(fd);
    return 0;
}
