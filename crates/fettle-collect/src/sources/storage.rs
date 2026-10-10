#[cfg(test)]
mod tests {
    use super::*;

    const DESC: usize = size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>();
    const SPECIFIC: usize = offset_of!(STORAGE_PROTOCOL_DATA_DESCRIPTOR, ProtocolSpecificData);
    const PROTO: usize = size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>();

    fn put(buf: &mut [u8], at: usize, v: u32) {
        buf[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// A reply buffer as the driver fills it: descriptor, then the log at ProtocolDataOffset.
    fn reply() -> Vec<u8> {
        let mut buf = vec![0u8; SPECIFIC + PROTO + LOG_LEN];
        put(&mut buf, 0, DESC as u32);
        put(&mut buf, 4, DESC as u32);
        put(&mut buf, SPECIFIC + offset_of!(STORAGE_PROTOCOL_SPECIFIC_DATA, ProtocolDataOffset), PROTO as u32);
        put(&mut buf, SPECIFIC + offset_of!(STORAGE_PROTOCOL_SPECIFIC_DATA, ProtocolDataLength), LOG_LEN as u32);
        buf[SPECIFIC + PROTO + 5] = 7;
        buf
    }

    #[test]
    fn extracts_log_from_valid_reply() {
        let buf = reply();
        let log = extract_health_log(&buf, buf.len()).unwrap();
        assert_eq!(log[5], 7);
    }

    #[test]
    fn rejects_reply_with_unfilled_descriptor() {
        let mut buf = reply();
        put(&mut buf, 0, 0);
        put(&mut buf, 4, 0);
        assert!(extract_health_log(&buf, buf.len()).is_err());
    }

    #[test]
    fn rejects_reply_shorter_than_log() {
        let buf = reply();
        assert!(extract_health_log(&buf, buf.len() - 1).is_err());
    }
}

use super::nvme::parse_nvme_health_log;
use serde_json::{Value, json};
use std::mem::{offset_of, size_of};
use windows::Win32::Foundation::{CloseHandle, ERROR_FILE_NOT_FOUND, GENERIC_READ, GENERIC_WRITE, HANDLE};
use windows::Win32::Storage::FileSystem::{CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING};
use windows::Win32::System::IO::DeviceIoControl;
use windows::Win32::System::Ioctl::{
    IOCTL_STORAGE_QUERY_PROPERTY, NVMeDataTypeLogPage, PropertyStandardQuery, ProtocolTypeNvme,
    STORAGE_PROPERTY_QUERY, STORAGE_PROTOCOL_DATA_DESCRIPTOR, STORAGE_PROTOCOL_SPECIFIC_DATA,
    StorageDeviceProtocolSpecificProperty,
};
use windows::core::HSTRING;

const NVME_LOG_PAGE_HEALTH: u32 = 0x02;
const LOG_LEN: usize = 512;
const MAX_DRIVES: u32 = 16;

/// NVMe health log for each physical drive, tried with no access rights and then with read/write.
pub fn nvme_health() -> Result<Value, String> {
    let mut out = Vec::new();
    for n in 0..MAX_DRIVES {
        let path = format!(r"\\.\PhysicalDrive{n}");
        let mut attempts = Vec::new();
        let mut missing = false;
        for (access_name, access) in [("none", 0u32), ("read", GENERIC_READ.0 | GENERIC_WRITE.0)] {
            match open_drive(&path, access) {
                Err(e) if e.code() == ERROR_FILE_NOT_FOUND.to_hresult() => {
                    missing = true;
                    break;
                }
                Err(e) => attempts.push(json!({"access": access_name, "ok": false, "error": format!("open: {e}")})),
                Ok(handle) => {
                    let result = query_health_log(handle);
                    unsafe {
                        let _ = CloseHandle(handle);
                    }
                    attempts.push(match result {
                        Ok(log) => json!({"access": access_name, "ok": true, "health": parse_nvme_health_log(&log)}),
                        Err(e) => json!({"access": access_name, "ok": false, "error": e}),
                    });
                }
            }
        }
        if !missing {
            out.push(json!({"drive": n, "attempts": attempts}));
        }
    }
    Ok(Value::Array(out))
}

fn open_drive(path: &str, access: u32) -> windows::core::Result<HANDLE> {
    unsafe {
        CreateFileW(
            &HSTRING::from(path),
            access,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            None,
        )
    }
}

/// Sends IOCTL_STORAGE_QUERY_PROPERTY for NVMe log page 0x02 and returns the 512 log bytes.
fn query_health_log(handle: HANDLE) -> Result<[u8; LOG_LEN], String> {
    let header = offset_of!(STORAGE_PROPERTY_QUERY, AdditionalParameters);
    let proto_size = size_of::<STORAGE_PROTOCOL_SPECIFIC_DATA>();
    let len = header + proto_size + LOG_LEN;
    // u64 storage keeps the buffer aligned for the structs written into it.
    let mut buf = vec![0u64; len.div_ceil(8)];
    let base = buf.as_mut_ptr() as *mut u8;
    unsafe {
        let query = &mut *(base as *mut STORAGE_PROPERTY_QUERY);
        query.PropertyId = StorageDeviceProtocolSpecificProperty;
        query.QueryType = PropertyStandardQuery;
        let proto = &mut *(base.add(header) as *mut STORAGE_PROTOCOL_SPECIFIC_DATA);
        proto.ProtocolType = ProtocolTypeNvme;
        proto.DataType = NVMeDataTypeLogPage.0 as u32;
        proto.ProtocolDataRequestValue = NVME_LOG_PAGE_HEALTH;
        proto.ProtocolDataRequestSubValue = 0;
        proto.ProtocolDataOffset = proto_size as u32;
        proto.ProtocolDataLength = LOG_LEN as u32;

        let mut returned = 0u32;
        DeviceIoControl(
            handle,
            IOCTL_STORAGE_QUERY_PROPERTY,
            Some(base as *const _),
            len as u32,
            Some(base as *mut _),
            len as u32,
            Some(&mut returned),
            None,
        )
        .map_err(|e| format!("ioctl: {e}"))?;

        extract_health_log(std::slice::from_raw_parts(base, len), returned as usize)
    }
}

fn read_u32(buf: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(buf[at..at + 4].try_into().unwrap())
}

/// Validates the driver's reply before trusting it. Input and output share one buffer,
/// so a driver that reports success without writing anything would otherwise leave
/// our own request fields in place and an all-zero "healthy" log behind them.
fn extract_health_log(buf: &[u8], returned: usize) -> Result<[u8; LOG_LEN], String> {
    let desc_size = size_of::<STORAGE_PROTOCOL_DATA_DESCRIPTOR>();
    let specific = offset_of!(STORAGE_PROTOCOL_DATA_DESCRIPTOR, ProtocolSpecificData);
    if buf.len() < desc_size {
        return Err("reply buffer too small".into());
    }
    let (version, size) = (read_u32(buf, 0) as usize, read_u32(buf, 4) as usize);
    if version != desc_size || size != desc_size {
        return Err(format!("descriptor not filled: version {version} size {size}"));
    }
    let data_offset = read_u32(buf, specific + offset_of!(STORAGE_PROTOCOL_SPECIFIC_DATA, ProtocolDataOffset)) as usize;
    let data_len = read_u32(buf, specific + offset_of!(STORAGE_PROTOCOL_SPECIFIC_DATA, ProtocolDataLength)) as usize;
    let start = specific + data_offset;
    if data_len < LOG_LEN || start + LOG_LEN > buf.len() || start + LOG_LEN > returned {
        return Err(format!("short log: offset {data_offset} length {data_len} returned {returned}"));
    }
    let mut log = [0u8; LOG_LEN];
    log.copy_from_slice(&buf[start..start + LOG_LEN]);
    Ok(log)
}
