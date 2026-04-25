// SPDX-License-Identifier: GPL-2.0
// SPDX-FileCopyrightText: Copyright (C) 2026 William Edwards <shadowapex@gmail.com>

//! Abstractions for the USB bus.
//!
//! C header: [`include/linux/hid.h`](srctree/include/linux/hid.h)

// OTHER SOURCE: https://lore.kernel.org/rust-for-linux/Z9MxI0u2yCfSzTvD@cassiopeiae/T/

use core::mem::MaybeUninit;

use crate::{
    device_id::{RawDeviceId, RawDeviceIdIndex},
    error::*,
    prelude::*,
    types::Opaque,
};

/// Abstraction for the HID device structure, i.e. [`struct hid_device`].
#[repr(transparent)]
pub struct Device(Opaque<bindings::hid_device>);

/// Abstraction for the HID device ID structure, i.e. [`struct hid_device_id`].
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct DeviceId(bindings::hid_device_id);

impl DeviceId {
    /// Equivalent to C's `HID_USB_DEVICE` macro.
    pub const fn from_id(vendor: u32, product: u32) -> Self {
        Self(bindings::hid_device_id {
            vendor,
            product,
            // SAFETY: It is safe to use all zeroes for the other fields of `hid_device_id`.
            ..unsafe { MaybeUninit::zeroed().assume_init() }
        })
    }
}

// SAFETY: `DeviceId` is a `#[repr(transparent)]` wrapper of `hid_device_id` and does not add
// additional invariants, so it's safe to transmute to `RawType`.
unsafe impl RawDeviceId for DeviceId {
    type RawType = bindings::hid_device_id;
}

// SAFETY: `DRIVER_DATA_OFFSET` is the offset to the `driver_info` field.
unsafe impl RawDeviceIdIndex for DeviceId {
    const DRIVER_DATA_OFFSET: usize = core::mem::offset_of!(bindings::hid_device_id, driver_data);

    fn index(&self) -> usize {
        self.0.driver_data
    }
}

/// [`IdTable`](kernel::device_id::IdTable) type for HID.
pub type IdTable<T> = &'static dyn kernel::device_id::IdTable<DeviceId, T>;

/// Create an HID `IdTable` with its alias for modpost.
#[macro_export]
macro_rules! hid_device_table {
    ($table_name:ident, $module_table_name:ident, $id_info_type: ty, $table_data: expr) => {
        const $table_name: $crate::device_id::IdArray<
            $crate::hid::DeviceId,
            $id_info_type,
            { $table_data.len() },
        > = $crate::device_id::IdArray::new($table_data);

        $crate::module_device_table!("hid", $module_table_name, $table_name);
    };
}

/// The HID driver trait
///
/// # Examples
///
///```
/// # use kernel::{bindings, device::Core, hid};
/// use kernel::prelude::*;
///
/// struct MyDriver;
///
/// kernel::hid_device_table!(
///     HID_TABLE,
///     MODULE_HID_TABLE,
///     <MyDriver as hid::Driver>::IdInfo,
///     [
///         (hid::DeviceId::from_id(0x1234, 0x5678), ()),
///         (hid::DeviceId::from_id(0xabcd, 0xef01), ()),
///     ]
/// );
///
/// impl hid::Driver for MyDriver {
///     type IdInfo = ();
///     const ID_TABLE: hid::IdTable<Self::IdInfo> = &HID_TABLE;
///
///     fn probe(
///         _interface: &usb::Interface<Core>,
///         _id: &usb::DeviceId,
///         _info: &Self::IdInfo,
///     ) -> impl PinInit<Self, Error> {
///         Err(ENODEV)
///     }
///
///     fn disconnect(_interface: &usb::Interface<Core>, _data: Pin<&Self>) {}
/// }
///```
pub trait Driver {
    /// The type holding information about each one of the device ids supported by the driver.
    type IdInfo: 'static;

    /// The table of device ids supported by the driver.
    const ID_TABLE: IdTable<Self::IdInfo>;

    /// driver name (e.g. "Footech_bar-wheel")
    const NAME: &'static CStr;

    /// HID driver probe.
    ///
    /// Called when a new HID interface is bound to this driver.
    /// Implementers should attempt to initialize the interface here.
    fn probe(device: &mut Device, id: &DeviceId, id_info: &Self::IdInfo) -> Result<()>;

    /// device removed (NULL if not a hot-plug capable driver)
    fn remove(device: &mut Device) {}

    /// if report in report_table, this hook is called (NULL means nop)
    fn raw_event(&mut self) {}
}

/*
 * struct hid_driver
 * @name: driver name (e.g. "Footech_bar-wheel")
 * @id_table: which devices is this driver for (must be non-NULL for probe
 * 	      to be called)
 * @dyn_list: list of dynamically added device ids
 * @dyn_lock: lock protecting @dyn_list
 * @match: check if the given device is handled by this driver
 * @probe: new device inserted
 * @remove: device removed (NULL if not a hot-plug capable driver)
 * @report_table: on which reports to call raw_event (NULL means all)
 * @raw_event: if report in report_table, this hook is called (NULL means nop)
 * @usage_table: on which events to call event (NULL means all)
 * @event: if usage in usage_table, this hook is called (NULL means nop)
 * @report: this hook is called after parsing a report (NULL means nop)
 * @report_fixup: called before report descriptor parsing (NULL means nop)
 * @input_mapping: invoked on input registering before mapping an usage
 * @input_mapped: invoked on input registering after mapping an usage
 * @input_configured: invoked just before the device is registered
 * @feature_mapping: invoked on feature registering
 * @suspend: invoked on suspend (NULL means nop)
 * @resume: invoked on resume if device was not reset (NULL means nop)
 * @reset_resume: invoked on resume if device was reset (NULL means nop)
 * @on_hid_hw_open: invoked when hid core opens first instance (NULL means nop)
 * @on_hid_hw_close: invoked when hid core closes last instance (NULL means nop)
 *
 * probe should return -errno on error, or 0 on success. During probe,
 * input will not be passed to raw_event unless hid_device_io_start is
 * called.
 *
 * raw_event and event should return negative on error, any other value will
 * pass the event on to .event() typically return 0 for success.
 *
 * report_fixup must return a report descriptor pointer whose lifetime is at
 * least that of the input rdesc.  This is usually done by mutating the input
 * rdesc and returning it or a sub-portion of it.  In case a new buffer is
 * allocated and returned, the implementation of report_fixup is responsible for
 * freeing it later.
 *
 * input_mapping shall return a negative value to completely ignore this usage
 * (e.g. doubled or invalid usage), zero to continue with parsing of this
 * usage by generic code (no special handling needed) or positive to skip
 * generic parsing (needed special handling which was done in the hook already)
 * input_mapped shall return negative to inform the layer that this usage
 * should not be considered for further processing or zero to notify that
 * no processing was performed and should be done in a generic manner
 * Both these functions may be NULL which means the same behavior as returning
 * zero from them.
 */
//struct hid_driver {
//	const char *name;
//	const struct hid_device_id *id_table;
//
//	struct list_head dyn_list;
//	spinlock_t dyn_lock;
//
//	bool (*match)(struct hid_device *dev, bool ignore_special_driver);
//	int (*probe)(struct hid_device *dev, const struct hid_device_id *id);
//	void (*remove)(struct hid_device *dev);
//
//	const struct hid_report_id *report_table;
//	int (*raw_event)(struct hid_device *hdev, struct hid_report *report,
//			u8 *data, int size);
//	const struct hid_usage_id *usage_table;
//	int (*event)(struct hid_device *hdev, struct hid_field *field,
//			struct hid_usage *usage, __s32 value);
//	void (*report)(struct hid_device *hdev, struct hid_report *report);
//
//	const __u8 *(*report_fixup)(struct hid_device *hdev, __u8 *buf,
//			unsigned int *size);
//
//	int (*input_mapping)(struct hid_device *hdev,
//			struct hid_input *hidinput, struct hid_field *field,
//			struct hid_usage *usage, unsigned long **bit, int *max);
//	int (*input_mapped)(struct hid_device *hdev,
//			struct hid_input *hidinput, struct hid_field *field,
//			struct hid_usage *usage, unsigned long **bit, int *max);
//	int (*input_configured)(struct hid_device *hdev,
//				struct hid_input *hidinput);
//	void (*feature_mapping)(struct hid_device *hdev,
//			struct hid_field *field,
//			struct hid_usage *usage);
//
//	int (*suspend)(struct hid_device *hdev, pm_message_t message);
//	int (*resume)(struct hid_device *hdev);
//	int (*reset_resume)(struct hid_device *hdev);
//	void (*on_hid_hw_open)(struct hid_device *hdev);
//	void (*on_hid_hw_close)(struct hid_device *hdev);
//
//* private: */
//	struct device_driver driver;
//};
