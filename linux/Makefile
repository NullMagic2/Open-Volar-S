obj-m += open_volar_s_usb.o
KDIR ?= /lib/modules/$(shell uname -r)/build

.PHONY: all clean install-permissions
all:
	$(MAKE) -C $(KDIR) M=$(CURDIR) modules

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR) clean

# Also available separately; build.sh calls this helper automatically.
install-permissions:
	bash "$(CURDIR)/install_permissions.sh"
