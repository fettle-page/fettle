/// (source name, namespace, WQL). Each row becomes one collector.
pub const WMI_SOURCES: &[(&str, &str, &str)] = &[
    ("os", CIMV2, "SELECT * FROM Win32_OperatingSystem"),
    ("computer_system", CIMV2, "SELECT * FROM Win32_ComputerSystem"),
    ("cpu", CIMV2, "SELECT * FROM Win32_Processor"),
    ("baseboard", CIMV2, "SELECT * FROM Win32_BaseBoard"),
    ("bios", CIMV2, "SELECT * FROM Win32_BIOS"),
    ("memory_modules", CIMV2, "SELECT * FROM Win32_PhysicalMemory"),
    ("memory_array", CIMV2, "SELECT * FROM Win32_PhysicalMemoryArray"),
    ("video_controllers", CIMV2, "SELECT * FROM Win32_VideoController"),
    ("monitor_ids", WMI, "SELECT * FROM WmiMonitorID"),
    // Explicit columns: SupportedDisplayFeatures is an embedded object the wmi crate can't decode.
    (
        "monitor_params",
        WMI,
        "SELECT InstanceName, Active, MaxHorizontalImageSize, MaxVerticalImageSize, VideoInputType \
         FROM WmiMonitorBasicDisplayParams",
    ),
    ("logical_disks", CIMV2, "SELECT * FROM Win32_LogicalDisk WHERE DriveType = 3"),
    ("disk_drives", CIMV2, "SELECT * FROM Win32_DiskDrive"),
    ("physical_disks", STORAGE, "SELECT * FROM MSFT_PhysicalDisk"),
    ("reliability_direct", STORAGE, "SELECT * FROM MSFT_StorageReliabilityCounter"),
    // Explicit columns: Win32_Battery.DeviceID is usually serial + manufacturer + name.
    (
        "battery_cimv2",
        CIMV2,
        "SELECT Name, Caption, Status, Availability, BatteryStatus, Chemistry, DesignCapacity, DesignVoltage, \
         FullChargeCapacity, EstimatedChargeRemaining, EstimatedRunTime, ExpectedLife FROM Win32_Battery",
    ),
    // Raw ATA SMART attribute table (reallocated/pending sectors live here, not in the
    // reliability counters). VendorSpecific is the 512-byte attribute block.
    ("smart_predict_data", WMI, "SELECT * FROM MSStorageDriver_FailurePredictData"),
    ("smart_predict_status", WMI, "SELECT * FROM MSStorageDriver_FailurePredictStatus"),
    ("battery_static", WMI, "SELECT * FROM BatteryStaticData"),
    ("battery_full_capacity", WMI, "SELECT * FROM BatteryFullChargedCapacity"),
    ("battery_cycle_count", WMI, "SELECT * FROM BatteryCycleCount"),
    ("battery_status", WMI, "SELECT * FROM BatteryStatus"),
    ("hotfixes", CIMV2, "SELECT * FROM Win32_QuickFixEngineering"),
    ("tpm", TPM, "SELECT * FROM Win32_Tpm"),
    ("startup_wmi", CIMV2, "SELECT * FROM Win32_StartupCommand"),
    // Explicit columns so no product-key field is ever selected.
    (
        "esu_licensing",
        CIMV2,
        "SELECT Name, Description, LicenseStatus, GracePeriodRemaining FROM SoftwareLicensingProduct \
         WHERE Name LIKE '%ESU%' OR Description LIKE '%ESU%'",
    ),
];

const CIMV2: &str = r"ROOT\CIMV2";
const WMI: &str = r"ROOT\WMI";
const STORAGE: &str = r"ROOT\Microsoft\Windows\Storage";
const TPM: &str = r"ROOT\CIMV2\Security\MicrosoftTpm";
