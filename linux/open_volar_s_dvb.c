// SPDX-License-Identifier: GPL-3.0-only
/* Included by the USB translation unit. DVB core supplies the standard frontend,
 * demux and DVR API; the broker reuses the established Rust IT9175 controller. */
#include "open_volar_s_dvb.h"
#if IS_REACHABLE(CONFIG_DVB_CORE)
#include <linux/miscdevice.h>
#include <linux/jiffies.h>
#include <media/dvb_frontend.h>
#include <media/dvb_demux.h>
#include <media/dmxdev.h>

struct ovs_dvb {
    struct kref ref;
    struct mutex lock;
    bool disconnected, broker_open;
    struct ovs_dvb_request request;
    enum fe_status status;
    unsigned long status_at;
    struct dvb_adapter adapter;
    struct dvb_frontend frontend;
    struct dvb_demux demux;
    struct dmxdev dmxdev;
    struct dmx_frontend input;
    struct miscdevice broker;
    char name[40];
};
DVB_DEFINE_MOD_OPT_ADAPTER_NR(adapter_nr);
/* Compatibility alias for applications whose capture UI omits ISDB-T.
 * This changes the tuning API only, never the RF standard or TS data path.
 * Keep ISDB-T first so applications that detect delivery systems prefer it.
 */
static bool dvbt_compat = true;
module_param(dvbt_compat, bool, 0444);
MODULE_PARM_DESC(dvbt_compat,
    "Accept DVB-T tuning as an alias for 6 MHz ISDB-T (default enabled; no DVB-T RF reception)");

static void ovs_dvb_free(struct kref *ref)
{
    kfree(container_of(ref, struct ovs_dvb, ref));
}
static int ovs_dvb_sleep(struct dvb_frontend *fe)
{
    struct ovs_dvb *d = fe->demodulator_priv;
    mutex_lock(&d->lock);
    d->request.active = 0;
    d->request.generation++;
    d->status = 0;
    mutex_unlock(&d->lock);
    return 0;
}
static int ovs_dvb_set_frontend(struct dvb_frontend *fe)
{
    struct ovs_dvb *d = fe->demodulator_priv;
    struct dtv_frontend_properties *p = &fe->dtv_property_cache;
    if ((p->delivery_system != SYS_ISDBT &&
         !(dvbt_compat && p->delivery_system == SYS_DVBT)) ||
        (p->bandwidth_hz && p->bandwidth_hz != 6000000) ||
        p->frequency < 470000000 || p->frequency > 697999000)
        return -EINVAL;
    mutex_lock(&d->lock);
    if (d->disconnected) {
        mutex_unlock(&d->lock);
        return -ENODEV;
    }
    d->request.frequency_hz = p->frequency;
    d->request.active = 1;
    d->request.generation++;
    d->status = 0;
    p->bandwidth_hz = 6000000;
    mutex_unlock(&d->lock);
    return 0;
}
static int ovs_dvb_read_status(struct dvb_frontend *fe, enum fe_status *status)
{
    struct ovs_dvb *d = fe->demodulator_priv;
    mutex_lock(&d->lock);
    *status = !d->disconnected && d->broker_open && d->request.active &&
        time_before(jiffies, d->status_at + 2 * HZ) ? d->status : 0;
    mutex_unlock(&d->lock);
    return 0;
}
static enum dvbfe_algo ovs_dvb_algo(struct dvb_frontend *fe)
{
    return DVBFE_ALGO_HW;
}
static int ovs_dvb_tune(struct dvb_frontend *fe, bool re_tune,
                        unsigned int mode, unsigned int *delay,
                        enum fe_status *status)
{
    int ret;
    *delay = HZ / 5;
    if (re_tune) {
        ret = ovs_dvb_set_frontend(fe);
        if (ret)
            return ret;
    }
    return ovs_dvb_read_status(fe, status);
}
static int ovs_dvb_start_feed(struct dvb_demux_feed *feed)
{
    struct ovs_dvb *d = feed->demux->priv;
    return READ_ONCE(d->disconnected) ? -ENODEV : 0;
}
static int ovs_dvb_stop_feed(struct dvb_demux_feed *feed) { return 0; }
static int ovs_broker_open(struct inode *inode, struct file *file)
{
    struct ovs_dvb *d = container_of(file->private_data, struct ovs_dvb, broker);
    int ret = 0;
    mutex_lock(&d->lock);
    if (d->disconnected)
        ret = -ENODEV;
    else if (d->broker_open)
        ret = -EBUSY;
    else {
        kref_get(&d->ref);
        d->broker_open = true;
        file->private_data = d;
    }
    mutex_unlock(&d->lock);
    return ret;
}
static int ovs_broker_release(struct inode *inode, struct file *file)
{
    struct ovs_dvb *d = file->private_data;
    mutex_lock(&d->lock);
    d->broker_open = false;
    d->status = 0;
    mutex_unlock(&d->lock);
    kref_put(&d->ref, ovs_dvb_free);
    return 0;
}
static long ovs_broker_ioctl(struct file *file, unsigned int cmd, unsigned long arg)
{
    struct ovs_dvb *d = file->private_data;
    struct ovs_dvb_status status;
    void __user *user = (void __user *)arg;
    int ret = 0;
    mutex_lock(&d->lock);
    if (d->disconnected) {
        ret = -ENODEV;
        goto done;
    }
    switch (cmd) {
    case OVS_DVB_GET_REQUEST:
        if (copy_to_user(user, &d->request, sizeof(d->request)))
            ret = -EFAULT;
        break;
    case OVS_DVB_SET_STATUS:
        if (copy_from_user(&status, user, sizeof(status))) {
            ret = -EFAULT;
            break;
        }
        if (status.generation != d->request.generation || !d->request.active) {
            ret = -ESTALE;
            break;
        }
        d->status = status.status & (FE_HAS_SIGNAL | FE_HAS_CARRIER |
            FE_HAS_VITERBI | FE_HAS_SYNC | FE_HAS_LOCK);
        d->status_at = jiffies;
        break;
    default:
        ret = -ENOTTY;
    }
done:
    mutex_unlock(&d->lock);
    return ret;
}
static ssize_t ovs_broker_write(struct file *file, const char __user *buf,
                               size_t count, loff_t *offset)
{
    struct ovs_dvb *d = file->private_data;
    u8 *data;
    u32 generation;
    ssize_t ret = count;
    if (count <= sizeof(generation) || count > 65536)
        return -EINVAL;
    data = memdup_user(buf, count);
    if (IS_ERR(data))
        return PTR_ERR(data);
    memcpy(&generation, data, sizeof(generation));
    mutex_lock(&d->lock);
    if (d->disconnected)
        ret = -ENODEV;
    else if (generation != d->request.generation || !d->request.active)
        ret = -ESTALE;
    else
        dvb_dmx_swfilter(&d->demux, data + sizeof(generation), count - sizeof(generation));
    mutex_unlock(&d->lock);
    kfree(data);
    return ret;
}
static const struct file_operations ovs_broker_fops = {
    .owner = THIS_MODULE, .open = ovs_broker_open, .release = ovs_broker_release,
    .unlocked_ioctl = ovs_broker_ioctl, .compat_ioctl = ovs_broker_ioctl,
    .write = ovs_broker_write, .llseek = noop_llseek,
};
static const struct dvb_frontend_ops ovs_dvb_ops = {
    .delsys = { SYS_ISDBT },
    .info = {
        .name = "Open Volar S A865R ISDB-T",
        .frequency_min_hz = 470000000, .frequency_max_hz = 697999000,
        .frequency_stepsize_hz = 1000,
        .caps = FE_CAN_INVERSION_AUTO | FE_CAN_FEC_AUTO | FE_CAN_QAM_AUTO |
            FE_CAN_TRANSMISSION_MODE_AUTO | FE_CAN_GUARD_INTERVAL_AUTO |
            FE_CAN_HIERARCHY_AUTO | FE_CAN_RECOVER,
    },
    .sleep = ovs_dvb_sleep, .set_frontend = ovs_dvb_set_frontend,
    .read_status = ovs_dvb_read_status, .tune = ovs_dvb_tune,
    .get_frontend_algo = ovs_dvb_algo,
};
static struct ovs_dvb *ovs_dvb_register(struct device *parent, unsigned int index)
{
    struct ovs_dvb *d = kzalloc(sizeof(*d), GFP_KERNEL);
    int ret;
    if (!d)
        return ERR_PTR(-ENOMEM);
    kref_init(&d->ref);
    mutex_init(&d->lock);
    d->request.device_index = index;
    ret = dvb_register_adapter(&d->adapter, "Open Volar S A865R ISDB-T", THIS_MODULE, parent, adapter_nr);
    if (ret < 0)
        goto free;
    d->frontend.ops = ovs_dvb_ops;
    if (dvbt_compat)
        d->frontend.ops.delsys[1] = SYS_DVBT;
    d->frontend.demodulator_priv = d;
    ret = dvb_register_frontend(&d->adapter, &d->frontend);
    if (ret)
        goto adapter;
    d->demux.dmx.capabilities = DMX_TS_FILTERING | DMX_SECTION_FILTERING | DMX_MEMORY_BASED_FILTERING;
    d->demux.priv = d;
    d->demux.filternum = 256;
    d->demux.feednum = 256;
    d->demux.start_feed = ovs_dvb_start_feed;
    d->demux.stop_feed = ovs_dvb_stop_feed;
    ret = dvb_dmx_init(&d->demux);
    if (ret)
        goto frontend;
    d->dmxdev.filternum = 256;
    d->dmxdev.demux = &d->demux.dmx;
    ret = dvb_dmxdev_init(&d->dmxdev, &d->adapter);
    if (ret)
        goto demux;
    d->input.source = DMX_FRONTEND_0;
    ret = d->demux.dmx.add_frontend(&d->demux.dmx, &d->input);
    if (ret)
        goto dmxdev;
    ret = d->demux.dmx.connect_frontend(&d->demux.dmx, &d->input);
    if (ret)
        goto input;
    snprintf(d->name, sizeof(d->name), "open-volar-dvb%u", index);
    d->broker.minor = MISC_DYNAMIC_MINOR;
    d->broker.name = d->name;
    d->broker.fops = &ovs_broker_fops;
    d->broker.parent = parent;
    d->broker.mode = 0600;
    ret = misc_register(&d->broker);
    if (!ret)
        return d;
    d->demux.dmx.disconnect_frontend(&d->demux.dmx);
input:
    d->demux.dmx.remove_frontend(&d->demux.dmx, &d->input);
dmxdev:
    dvb_dmxdev_release(&d->dmxdev);
demux:
    dvb_dmx_release(&d->demux);
frontend:
    dvb_unregister_frontend(&d->frontend);
adapter:
    dvb_unregister_adapter(&d->adapter);
free:
    kref_put(&d->ref, ovs_dvb_free);
    return ERR_PTR(ret);
}
static void ovs_dvb_unregister(struct ovs_dvb *d)
{
    if (!d)
        return;
    mutex_lock(&d->lock);
    d->disconnected = true;
    d->request.active = 0;
    mutex_unlock(&d->lock);
    misc_deregister(&d->broker);
    dvb_unregister_frontend(&d->frontend);
    d->demux.dmx.disconnect_frontend(&d->demux.dmx);
    d->demux.dmx.remove_frontend(&d->demux.dmx, &d->input);
    dvb_dmxdev_release(&d->dmxdev);
    dvb_dmx_release(&d->demux);
    dvb_unregister_adapter(&d->adapter);
    kref_put(&d->ref, ovs_dvb_free);
}
#else
struct ovs_dvb;
static struct ovs_dvb *ovs_dvb_register(struct device *parent, unsigned int index)
{
    dev_warn(parent, "DVB core unavailable; only the raw USB interface is enabled\n");
    return NULL;
}
static void ovs_dvb_unregister(struct ovs_dvb *d) { }
#endif
