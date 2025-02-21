use crate::common::*;
use std::convert::TryFrom;

use std::error::Error;
use udev::Enumerator;

pub fn enumerate_platform(vid: Option<u16>, pid: Option<u16>) -> Vec<UsbDevice> {
    let mut output = Vec::new();

    let mut enumerator = Enumerator::new().expect("could not get udev enumerator");

    for device in enumerator.scan_devices().expect("could not scan devices") {
        let _ = || -> Result<(), Box<dyn Error>> {
            let vendor_id = get_pid_or_vid(
                device
                    .property_value("ID_VENDOR_ID")
                    .ok_or(ParseError)?
                    .to_str()
                    .ok_or(ParseError)?,
            )?;

            if let Some(vid) = vid {
                if vid != vendor_id {
                    return Ok(());
                }
            }

            let product_id = get_pid_or_vid(
                device
                    .property_value("ID_MODEL_ID")
                    .ok_or(ParseError)?
                    .to_str()
                    .ok_or(ParseError)?,
            )?;

            if let Some(pid) = pid {
                if pid != product_id {
                    return Ok(());
                }
            }

            let id = device
                .property_value("DEVPATH")
                .ok_or(ParseError)?
                .to_str()
                .ok_or(ParseError)?
                .to_string();

            let mut description = device
                .property_value("ID_MODEL_FROM_DATABASE")
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());

            if description.is_none() {
                description = device
                    .property_value("ID_MODEL")
                    .and_then(|s| s.to_str())
                    .map(|s| s.to_string());
            }

            let serial_number = device
                .property_value("ID_SERIAL_SHORT")
                .and_then(|s| s.to_str())
                .map(|s| s.to_string());

            let bclass = device
                .attribute_value("bDeviceClass")
                .and_then(|x| x.to_str()?.parse::<u8>().ok());

            let friendly_name = device
                .property_value("ID_MODEL_FROM_DATABASE")
                .and_then(|s| s.to_str())
                .map(|x| x.to_string());
            let manufacturer = device
                .property_value("ID_VENDOR_FROM_DATABASE")
                .and_then(|s| s.to_str())
                .map(|x| x.to_string());
            let class = device
                .property_value("ID_PCI_CLASS_FROM_DATABASE")
                .and_then(|s| s.to_str())
                .map(|x| x.to_string());

            output.push(UsbDevice {
                id,
                vendor_id,
                product_id,
                description,
                serial_number,
                base_class: bclass.and_then(|bc| DeviceBaseClass::try_from(bc).ok()),
                class,
                friendly_name,
                manufacturer,
            });

            Ok(())
        }();
    }

    output
}

fn get_pid_or_vid(id: &str) -> Result<u16, Box<dyn Error>> {
    let mut id = id;
    // Sometimes they are prefixed
    if id.starts_with("0x") {
        id = &id[2..];
    }

    Ok(u16::from_str_radix(&id, 16)?)
}
