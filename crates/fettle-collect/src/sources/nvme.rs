use serde::Serialize;

/// The fields Fettle cares about from the NVMe SMART / Health Information log (page 0x02).
#[derive(Serialize, Debug, PartialEq)]
pub struct NvmeHealth {
    pub critical_warning: u8,
    pub temperature_c: Option<i32>,
    pub available_spare_pct: u8,
    pub available_spare_threshold_pct: u8,
    pub percentage_used: u8,
    pub data_units_read: u64,
    pub data_units_written: u64,
    pub power_cycles: u64,
    pub power_on_hours: u64,
    pub unsafe_shutdowns: u64,
    pub media_errors: u64,
    pub error_log_entries: u64,
}

/// Low 8 bytes of a 16-byte little-endian counter. Real values never exceed u64.
fn counter(log: &[u8; 512], offset: usize) -> u64 {
    u64::from_le_bytes(log[offset..offset + 8].try_into().unwrap())
}

pub fn parse_nvme_health_log(log: &[u8; 512]) -> NvmeHealth {
    let kelvin = u16::from_le_bytes([log[1], log[2]]);
    NvmeHealth {
        critical_warning: log[0],
        temperature_c: (kelvin != 0).then(|| kelvin as i32 - 273),
        available_spare_pct: log[3],
        available_spare_threshold_pct: log[4],
        percentage_used: log[5],
        data_units_read: counter(log, 32),
        data_units_written: counter(log, 48),
        power_cycles: counter(log, 112),
        power_on_hours: counter(log, 128),
        unsafe_shutdowns: counter(log, 144),
        media_errors: counter(log, 160),
        error_log_entries: counter(log, 176),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_health_log() {
        let mut log = [0u8; 512];
        log[0] = 0x04;
        log[1..3].copy_from_slice(&318u16.to_le_bytes());
        log[3] = 100;
        log[4] = 10;
        log[5] = 3;
        log[48..56].copy_from_slice(&5_000_000u64.to_le_bytes());
        log[112..120].copy_from_slice(&812u64.to_le_bytes());
        log[128..136].copy_from_slice(&1234u64.to_le_bytes());
        log[144..152].copy_from_slice(&41u64.to_le_bytes());
        log[160..168].copy_from_slice(&2u64.to_le_bytes());
        assert_eq!(
            parse_nvme_health_log(&log),
            NvmeHealth {
                critical_warning: 4,
                temperature_c: Some(45),
                available_spare_pct: 100,
                available_spare_threshold_pct: 10,
                percentage_used: 3,
                data_units_read: 0,
                data_units_written: 5_000_000,
                power_cycles: 812,
                power_on_hours: 1234,
                unsafe_shutdowns: 41,
                media_errors: 2,
                error_log_entries: 0,
            }
        );
    }

    #[test]
    fn zero_kelvin_means_no_temperature() {
        assert_eq!(parse_nvme_health_log(&[0u8; 512]).temperature_c, None);
    }

    #[test]
    fn wear_over_100_kept_raw() {
        let mut l = [0u8; 512];
        l[5] = 255;
        assert_eq!(parse_nvme_health_log(&l).percentage_used, 255);
    }
}
