/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef OPEN_VOLAR_S_DVB_H
#define OPEN_VOLAR_S_DVB_H
#include <linux/ioctl.h>
#include <linux/types.h>
/* Fixed-width broker ABI. Frequency is Hz, matching the DVB API. */
struct ovs_dvb_request {
    __u32 generation;
    __u32 frequency_hz;
    __u32 active;
    __u32 device_index;
};
struct ovs_dvb_status {
    __u32 generation;
    __u32 status; /* enum fe_status, never fabricated by the kernel bridge */
};
#define OVS_DVB_GET_REQUEST _IOR('V', 16, struct ovs_dvb_request)
#define OVS_DVB_SET_STATUS _IOW('V', 17, struct ovs_dvb_status)
/* write(): native-endian generation u32 followed by MPEG-TS bytes. */
#endif
