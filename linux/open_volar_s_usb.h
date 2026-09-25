/* SPDX-License-Identifier: GPL-3.0-only */
#ifndef OPEN_VOLAR_S_USB_H
#define OPEN_VOLAR_S_USB_H

#include <linux/ioctl.h>
#include <linux/types.h>

#define OVS_USB_ABI_VERSION 1
#define OVS_USB_MAX_ENDPOINTS 16
#define OVS_USB_MAX_TRANSFER 16384

struct ovs_usb_endpoint {
    __u8 address;
    __u8 reserved;
    __u16 max_packet_size;
};

struct ovs_usb_info {
    __u32 abi_version;
    __u16 vendor_id;
    __u16 product_id;
    __u8 interface_number;
    __u8 endpoint_count;
    __u8 reserved[2];
    struct ovs_usb_endpoint endpoints[OVS_USB_MAX_ENDPOINTS];
};

/* data is a userspace pointer, represented as an aligned 64-bit integer. */
struct ovs_usb_bulk {
    __aligned_u64 data;
    __u32 length;
    __u32 timeout_ms;
    __u32 actual_length;
    __u8 endpoint;
    __u8 reserved[3];
};

#define OVS_USB_GET_INFO _IOR('V', 1, struct ovs_usb_info)
#define OVS_USB_BULK _IOWR('V', 2, struct ovs_usb_bulk)

#endif
