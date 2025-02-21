use crate::common::{ParseError, UsbDevice};
use std::{
    error::Error,
    mem::{size_of, zeroed},
    ptr::{null, null_mut},
};
use windows_sys::{
    w,
    Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceRegistryPropertyW, DIGCF_ALLCLASSES,
        DIGCF_PRESENT, SPDRP_CLASS, SPDRP_DEVICEDESC, SPDRP_FRIENDLYNAME, SPDRP_HARDWAREID,
        SPDRP_MFG, SP_DEVINFO_DATA,
    },
};

pub fn enumerate_platform(vid: Option<u16>, pid: Option<u16>) -> Vec<UsbDevice> {
    let mut output: Vec<UsbDevice> = Vec::new();
    let usb = w!("USB\0");
    let dev_info =
        unsafe { SetupDiGetClassDevsW(null(), usb, -1, DIGCF_ALLCLASSES | DIGCF_PRESENT) };

    let mut dev_info_data = unsafe {
        SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ClassGuid: zeroed(),
            DevInst: zeroed(),
            Reserved: zeroed(),
        }
    };

    let mut i = 0;
    while unsafe { SetupDiEnumDeviceInfo(dev_info, i, &mut dev_info_data) } > 0 {
        i += 1;
        let mut buf: Vec<u8> = vec![0; 1000];

        if unsafe {
            SetupDiGetDeviceRegistryPropertyW(
                dev_info,
                &mut dev_info_data,
                SPDRP_HARDWAREID,
                null_mut(),
                buf.as_mut_ptr(),
                buf.len() as u32,
                null_mut(),
            )
        } > 0
        {
            if let Ok((vendor_id, product_id)) = extract_vid_pid(buf) {
                if let Some(vid) = vid {
                    if vid != vendor_id {
                        continue;
                    }
                }

                if let Some(pid) = pid {
                    if pid != product_id {
                        continue;
                    }
                }

                buf = vec![0; 1000];

                let mut class = None;
                if unsafe {
                    SetupDiGetDeviceRegistryPropertyW(
                        dev_info,
                        &mut dev_info_data,
                        SPDRP_CLASS,
                        null_mut(),
                        buf.as_mut_ptr(),
                        buf.len() as u32,
                        null_mut(),
                    )
                } > 0
                {
                    class = Some(string_from_buf_u8(buf));
                }

                // manufactor
                buf = vec![0; 1000];
                let mut manufacturer = None;
                if unsafe {
                    SetupDiGetDeviceRegistryPropertyW(
                        dev_info,
                        &mut dev_info_data,
                        SPDRP_MFG,
                        null_mut(),
                        buf.as_mut_ptr(),
                        buf.len() as u32,
                        null_mut(),
                    )
                } > 0
                {
                    manufacturer = Some(string_from_buf_u8(buf));
                }

                // friendly name
                buf = vec![0; 1000];
                let mut friendly_name = None;
                if unsafe {
                    SetupDiGetDeviceRegistryPropertyW(
                        dev_info,
                        &mut dev_info_data,
                        SPDRP_FRIENDLYNAME,
                        null_mut(),
                        buf.as_mut_ptr(),
                        buf.len() as u32,
                        null_mut(),
                    )
                } > 0
                {
                    friendly_name = Some(string_from_buf_u8(buf));
                }

                buf = vec![0; 1000];

                if unsafe {
                    SetupDiGetDeviceRegistryPropertyW(
                        dev_info,
                        &mut dev_info_data,
                        SPDRP_DEVICEDESC,
                        null_mut(),
                        buf.as_mut_ptr(),
                        buf.len() as u32,
                        null_mut(),
                    )
                } > 0
                {
                    let description = string_from_buf_u8(buf);

                    let mut buf: Vec<u16> = vec![0; 1000];

                    if unsafe {
                        SetupDiGetDeviceInstanceIdW(
                            dev_info,
                            &mut dev_info_data,
                            buf.as_mut_ptr(),
                            buf.len() as u32,
                            null_mut(),
                        )
                    } > 0
                    {
                        let id = string_from_buf_u16(buf.clone());
                        let serial_number = extract_serial_number(buf);
                        output.push(UsbDevice {
                            id,
                            vendor_id,
                            product_id,
                            friendly_name,
                            manufacturer,
                            description: Some(description),
                            serial_number,
                            class: class.clone(),
                            base_class: class.map(|cls| cls.into()),
                        });
                    }
                }
            }
        }
    }

    unsafe { SetupDiDestroyDeviceInfoList(dev_info) };

    output
}

fn extract_vid_pid(buf: Vec<u8>) -> Result<(u16, u16), Box<dyn Error + Send + Sync>> {
    let id = string_from_buf_u8(buf).to_uppercase();

    let vid = id.find("VID_").ok_or(ParseError)?;
    let pid = id.find("PID_").ok_or(ParseError)?;

    Ok((
        u16::from_str_radix(&id[vid + 4..vid + 8], 16)?,
        u16::from_str_radix(&id[pid + 4..pid + 8], 16)?,
    ))
}

fn extract_serial_number(buf: Vec<u16>) -> Option<String> {
    let id = string_from_buf_u16(buf);

    id.split('\\').last().map(std::borrow::ToOwned::to_owned)
}

fn string_from_buf_u16(buf: Vec<u16>) -> String {
    let mut out = String::from_utf16_lossy(&buf);

    if let Some(i) = out.find('\u{0}') {
        out.truncate(i);
    }

    out
}

fn string_from_buf_u8(buf: Vec<u8>) -> String {
    let str_vec: Vec<u16> = buf
        .chunks_exact(2)
        .map(|a| u16::from_ne_bytes([a[0], a[1]]))
        .collect();

    string_from_buf_u16(str_vec)
}
