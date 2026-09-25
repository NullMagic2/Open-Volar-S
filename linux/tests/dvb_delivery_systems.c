/* Query real frontend discovery without tuning or capturing the receiver.
 * cc -Wall -Wextra -Werror -o /tmp/ovs-delsys linux/tests/dvb_delivery_systems.c
 * /tmp/ovs-delsys /dev/dvb/adapter0/frontend0 1  # compatibility enabled
 */
#include <fcntl.h>
#include <linux/dvb/frontend.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/ioctl.h>
#include <unistd.h>

int main(int argc, char **argv)
{
    struct dtv_property property = { .cmd = DTV_ENUM_DELSYS };
    struct dtv_properties properties = { .num = 1, .props = &property };
    int fd, expected, isdb = 0, dvbt = 0;
    if (argc != 3 || (argv[2][0] != '0' && argv[2][0] != '1') || argv[2][1]) {
        fprintf(stderr, "Usage: %s FRONTEND COMPATIBILITY_0_OR_1\n", argv[0]);
        return 2;
    }
    expected = atoi(argv[2]);
    fd = open(argv[1], O_RDONLY | O_NONBLOCK);
    if (fd < 0) { perror("open frontend"); return 1; }
    if (ioctl(fd, FE_GET_PROPERTY, &properties) < 0) {
        perror("DTV_ENUM_DELSYS"); close(fd); return 1;
    }
    close(fd);
    if (!property.u.buffer.len || property.u.buffer.data[0] != SYS_ISDBT) {
        fprintf(stderr, "ISDB-T must remain the preferred delivery system\n");
        return 1;
    }
    for (unsigned i = 0; i < property.u.buffer.len; ++i) {
        unsigned system = property.u.buffer.data[i];
        printf("delivery system: %u%s\n", system,
               system == SYS_ISDBT ? " (ISDB-T)" : system == SYS_DVBT ? " (DVB-T alias)" : "");
        isdb += system == SYS_ISDBT;
        dvbt += system == SYS_DVBT;
    }
    return !(isdb == 1 && dvbt == expected && property.u.buffer.len == 1u + expected);
}
