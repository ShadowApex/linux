// SPDX-License-Identifier: GPL-2.0

//! Rust minimal sample.

use kernel::prelude::*;
//use kernel::usb::Driver; // TODO: create this for hid!
use kernel::hid::{DeviceId, Driver, IdTable};

module! {
    type: HidUcis,
    name: "hid_ucis",
    authors: ["William Edwards"],
    description: "Unified Controller Input Device Driver",
    license: "GPL",
}

kernel::hid_device_table!(
    HID_TABLE,
    MODULE_HID_TABLE,
    <HidUcis as Driver>::IdInfo,
    [(DeviceId::from_id(0x1d50, 0x616a), ()),]
);

struct HidUcis {
    numbers: KVec<i32>,
}

impl kernel::Module for HidUcis {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Rust minimal sample (init)\n");
        pr_info!("Am I built-in? {}\n", !cfg!(MODULE));

        let mut numbers = KVec::new();
        numbers.push(72, GFP_KERNEL)?;
        numbers.push(108, GFP_KERNEL)?;
        numbers.push(200, GFP_KERNEL)?;

        // TODO: Call hid_register_driver

        Ok(HidUcis { numbers })
    }
}

impl Driver for HidUcis {
    type IdInfo = ();

    const ID_TABLE: IdTable<Self::IdInfo> = &HID_TABLE;
    const NAME: &'static CStr = c"hid-ucis";

    fn probe(device: &mut Device, id: &DeviceId, id_info: &Self::IdInfo) -> Result<()> {
        pr_info!("Probe!");
        Ok(())
    }
}

impl Drop for HidUcis {
    fn drop(&mut self) {
        pr_info!("My numbers are {:?}\n", self.numbers);
        pr_info!("Rust minimal sample (exit)\n");
        pr_info!("bye world!\n");

        // TODO: call hid_unregister_driver
    }
}
