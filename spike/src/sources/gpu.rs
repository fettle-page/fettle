use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use serde_json::{Value, json};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, DXGI_ERROR_NOT_FOUND, IDXGIFactory1};

pub fn utf16_until_nul(s: &[u16]) -> String {
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    String::from_utf16_lossy(&s[..end])
}

/// Real VRAM per adapter. WMI's AdapterRAM is a 32-bit field and caps at 4 GB.
pub fn dxgi_adapters() -> Result<Value, String> {
    let factory: IDXGIFactory1 = unsafe { CreateDXGIFactory1() }.map_err(|e| format!("CreateDXGIFactory1: {e}"))?;
    let mut out = Vec::new();
    for i in 0.. {
        let adapter = match unsafe { factory.EnumAdapters1(i) } {
            Ok(a) => a,
            Err(e) if e.code() == DXGI_ERROR_NOT_FOUND => break,
            Err(e) => return Err(format!("EnumAdapters1({i}): {e}")),
        };
        let desc = unsafe { adapter.GetDesc1() }.map_err(|e| format!("GetDesc1({i}): {e}"))?;
        out.push(json!({
            "description": utf16_until_nul(&desc.Description),
            "vendor_id": desc.VendorId,
            "device_id": desc.DeviceId,
            "dedicated_video_memory": desc.DedicatedVideoMemory,
            "shared_system_memory": desc.SharedSystemMemory,
            "software": desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0,
        }));
    }
    Ok(Value::Array(out))
}

/// NVIDIA-only details. On other PCs Nvml::init fails, and that error is kept verbatim.
pub fn nvml() -> Result<Value, String> {
    let nvml = Nvml::init().map_err(|e| format!("NVML init: {e}"))?;
    let count = nvml.device_count().map_err(|e| format!("device_count: {e}"))?;
    let mut devices = Vec::new();
    for i in 0..count {
        let device = nvml.device_by_index(i).map_err(|e| format!("device_by_index({i}): {e}"))?;
        let memory = device.memory_info().ok();
        devices.push(json!({
            "name": device.name().ok(),
            "temperature_c": device.temperature(TemperatureSensor::Gpu).ok(),
            "memory_total": memory.as_ref().map(|m| m.total),
            "memory_used": memory.as_ref().map(|m| m.used),
        }));
    }
    Ok(json!({"driver_version": nvml.sys_driver_version().ok(), "devices": devices}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_stops_at_nul() {
        let mut buf = [0u16; 8];
        for (i, c) in "RTX".encode_utf16().enumerate() {
            buf[i] = c;
        }
        assert_eq!(utf16_until_nul(&buf), "RTX");
        assert_eq!(utf16_until_nul(&[]), "");
    }
}
