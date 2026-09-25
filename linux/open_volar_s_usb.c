// SPDX-License-Identifier: GPL-3.0-only
/* USB bulk character driver for the original AVerTV Volar S (A865R).
 * Framing, firmware and tuner control remain in the Rust userspace library.
 */
#include <linux/fs.h>
#include <linux/kref.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/slab.h>
#include <linux/uaccess.h>
#include <linux/usb.h>

#include "open_volar_s_usb.h"
#include "open_volar_s_dvb.c"

#define OVS_VENDOR_ID 0x07ca
#define OVS_PRODUCT_ID 0xb865

struct ovs_device {
    struct usb_device *udev;
    struct usb_interface *interface;
    struct mutex io_lock;
    struct kref kref;
    bool opened;
    struct ovs_dvb *dvb;
};

static struct usb_driver ovs_driver;
static DEFINE_MUTEX(ovs_disconnect_lock);

static void ovs_delete(struct kref *ref)
{
    struct ovs_device *dev = container_of(ref, struct ovs_device, kref);

    usb_put_dev(dev->udev);
    kfree(dev);
}

static int ovs_open(struct inode *inode, struct file *file)
{
    struct usb_interface *interface;
    struct ovs_device *dev;
    int result = 0;

    mutex_lock(&ovs_disconnect_lock);
    interface = usb_find_interface(&ovs_driver, iminor(inode));
    if (!interface) {
        result = -ENODEV;
        goto done;
    }
    dev = usb_get_intfdata(interface);
    if (!dev) {
        result = -ENODEV;
        goto done;
    }
    mutex_lock(&dev->io_lock);
    if (!dev->interface)
        result = -ENODEV;
    else if (dev->opened)
        result = -EBUSY;
    else {
        dev->opened = true;
        kref_get(&dev->kref);
        file->private_data = dev;
    }
    mutex_unlock(&dev->io_lock);
done:
    mutex_unlock(&ovs_disconnect_lock);
    return result;
}

static int ovs_release(struct inode *inode, struct file *file)
{
    struct ovs_device *dev = file->private_data;

    (void)inode;
    if (dev) {
        mutex_lock(&dev->io_lock);
        dev->opened = false;
        mutex_unlock(&dev->io_lock);
        kref_put(&dev->kref, ovs_delete);
    }
    return 0;
}

static bool ovs_valid_endpoint(struct ovs_device *dev, u8 address)
{
    struct usb_host_interface *alt = dev->interface->cur_altsetting;
    int index;

    for (index = 0; index < alt->desc.bNumEndpoints; index++) {
        const struct usb_endpoint_descriptor *ep = &alt->endpoint[index].desc;

        if (usb_endpoint_xfer_bulk(ep) && ep->bEndpointAddress == address)
            return true;
    }
    return false;
}

static int ovs_bulk(struct ovs_device *dev, void __user *argument)
{
    struct ovs_usb_bulk request;
    void __user *user_data;
    unsigned char *buffer;
    unsigned int pipe;
    int actual = 0;
    int result;

    if (copy_from_user(&request, argument, sizeof(request)))
        return -EFAULT;
    if (request.length > OVS_USB_MAX_TRANSFER || !request.timeout_ms ||
        !ovs_valid_endpoint(dev, request.endpoint))
        return -EINVAL;
    user_data = (void __user *)(unsigned long)request.data;
    if (request.length && !access_ok(user_data, request.length))
        return -EFAULT;
    buffer = kmalloc(max_t(u32, request.length, 1), GFP_KERNEL);
    if (!buffer)
        return -ENOMEM;
    if (!(request.endpoint & USB_DIR_IN) && request.length &&
        copy_from_user(buffer, user_data, request.length)) {
        result = -EFAULT;
        goto done;
    }
    pipe = request.endpoint & USB_DIR_IN
        ? usb_rcvbulkpipe(dev->udev, request.endpoint & USB_ENDPOINT_NUMBER_MASK)
        : usb_sndbulkpipe(dev->udev, request.endpoint & USB_ENDPOINT_NUMBER_MASK);
    result = usb_bulk_msg(dev->udev, pipe, buffer, request.length,
                          &actual, request.timeout_ms);
    if (result)
        goto done;
    if (actual < 0 || actual > request.length) {
        result = -EIO;
        goto done;
    }
    if ((request.endpoint & USB_DIR_IN) && actual &&
        copy_to_user(user_data, buffer, actual)) {
        result = -EFAULT;
        goto done;
    }
    request.actual_length = actual;
    if (copy_to_user(argument, &request, sizeof(request)))
        result = -EFAULT;
done:
    kfree(buffer);
    return result;
}

static long ovs_ioctl(struct file *file, unsigned int command, unsigned long argument)
{
    struct ovs_device *dev = file->private_data;
    void __user *user_argument = (void __user *)argument;
    struct ovs_usb_info info = { 0 };
    struct usb_host_interface *alt;
    int index;
    int result;

    if (mutex_lock_interruptible(&dev->io_lock))
        return -ERESTARTSYS;
    if (!dev->interface) {
        result = -ENODEV;
        goto done;
    }
    switch (command) {
    case OVS_USB_GET_INFO:
        info.abi_version = OVS_USB_ABI_VERSION;
        info.vendor_id = le16_to_cpu(dev->udev->descriptor.idVendor);
        info.product_id = le16_to_cpu(dev->udev->descriptor.idProduct);
        alt = dev->interface->cur_altsetting;
        info.interface_number = alt->desc.bInterfaceNumber;
        for (index = 0; index < alt->desc.bNumEndpoints; index++) {
            const struct usb_endpoint_descriptor *ep = &alt->endpoint[index].desc;
            struct ovs_usb_endpoint *out;

            if (!usb_endpoint_xfer_bulk(ep))
                continue;
            if (info.endpoint_count == OVS_USB_MAX_ENDPOINTS)
                break;
            out = &info.endpoints[info.endpoint_count++];
            out->address = ep->bEndpointAddress;
            out->max_packet_size = usb_endpoint_maxp(ep);
        }
        result = copy_to_user(user_argument, &info, sizeof(info)) ? -EFAULT : 0;
        break;
    case OVS_USB_BULK:
        result = ovs_bulk(dev, user_argument);
        break;
    default:
        result = -ENOTTY;
    }
done:
    mutex_unlock(&dev->io_lock);
    return result;
}

static const struct file_operations ovs_fops = {
    .owner = THIS_MODULE,
    .open = ovs_open,
    .release = ovs_release,
    .unlocked_ioctl = ovs_ioctl,
    .llseek = noop_llseek,
};

static struct usb_class_driver ovs_class = {
    .name = "open-volar-s%d",
    .fops = &ovs_fops,
    .minor_base = 192,
};

static int ovs_probe(struct usb_interface *interface,
                     const struct usb_device_id *id)
{
    struct ovs_device *dev;
    struct usb_host_interface *alt = interface->cur_altsetting;
    bool input = false, output = false;
    int index;
    int result;

    (void)id;
    for (index = 0; index < alt->desc.bNumEndpoints; index++) {
        const struct usb_endpoint_descriptor *ep = &alt->endpoint[index].desc;

        if (!usb_endpoint_xfer_bulk(ep))
            continue;
        if (usb_endpoint_dir_in(ep))
            input = true;
        else
            output = true;
    }
    if (!input || !output)
        return -ENODEV;
    dev = kzalloc(sizeof(*dev), GFP_KERNEL);
    if (!dev)
        return -ENOMEM;
    kref_init(&dev->kref);
    mutex_init(&dev->io_lock);
    dev->udev = usb_get_dev(interface_to_usbdev(interface));
    dev->interface = interface;
    usb_set_intfdata(interface, dev);
    result = usb_register_dev(interface, &ovs_class);
    if (result) {
        usb_set_intfdata(interface, NULL);
        kref_put(&dev->kref, ovs_delete);
        return result;
    }
    dev->dvb = ovs_dvb_register(&interface->dev, interface->minor - (IS_ENABLED(CONFIG_USB_DYNAMIC_MINORS) ? 0 : ovs_class.minor_base));
    if (IS_ERR(dev->dvb)) {
        result = PTR_ERR(dev->dvb);
        usb_deregister_dev(interface, &ovs_class);
        usb_set_intfdata(interface, NULL);
        kref_put(&dev->kref, ovs_delete);
        return result;
    }
    dev_info(&interface->dev, "A865R ready at /dev/open-volar-s%d\n",
             interface->minor - (IS_ENABLED(CONFIG_USB_DYNAMIC_MINORS) ? 0 : ovs_class.minor_base));
    return 0;
}

static void ovs_disconnect(struct usb_interface *interface)
{
    struct ovs_device *dev;

    mutex_lock(&ovs_disconnect_lock);
    dev = usb_get_intfdata(interface);
    usb_set_intfdata(interface, NULL);
    usb_deregister_dev(interface, &ovs_class);
    mutex_lock(&dev->io_lock);
    dev->interface = NULL;
    mutex_unlock(&dev->io_lock);
    mutex_unlock(&ovs_disconnect_lock);
    ovs_dvb_unregister(dev->dvb);
    kref_put(&dev->kref, ovs_delete);
}

static const struct usb_device_id ovs_ids[] = {
    { USB_DEVICE(OVS_VENDOR_ID, OVS_PRODUCT_ID) },
    { }
};
MODULE_DEVICE_TABLE(usb, ovs_ids);

static struct usb_driver ovs_driver = {
    .name = "open_volar_s_usb",
    .probe = ovs_probe,
    .disconnect = ovs_disconnect,
    .id_table = ovs_ids,
};
module_usb_driver(ovs_driver);

MODULE_AUTHOR("Open Volar S contributors");
MODULE_DESCRIPTION("AVerTV Volar S A865R ISDB-T DVB and USB driver");
MODULE_LICENSE("GPL");
