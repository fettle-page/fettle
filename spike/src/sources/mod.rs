mod gpu;
mod startup;
mod storage;
mod windows;
mod wmi_tables;

use crate::report::Collector;
use crate::wmi_util::wmi_query;

/// Every collector the spike runs, in order. Later tasks append here.
pub fn all() -> Vec<(String, Collector)> {
    let mut list: Vec<(String, Collector)> = Vec::new();
    for &(name, namespace, wql) in wmi_tables::WMI_SOURCES {
        list.push((name.into(), Box::new(move || wmi_query(namespace, wql))));
    }
    list.push(("reliability_assoc".into(), Box::new(storage::reliability_assoc)));
    list.push(("nvme_health".into(), Box::new(storage::nvme_health)));
    list.push(("windows_registry".into(), Box::new(windows::windows_registry)));
    list.push(("secure_boot".into(), Box::new(windows::secure_boot)));
    list.push(("esu_registry_search".into(), Box::new(windows::esu_registry_search)));
    list.push(("startup_registry".into(), Box::new(startup::startup_registry)));
    list.push(("startup_folders".into(), Box::new(startup::startup_folders)));
    list.push(("startup_approved".into(), Box::new(startup::startup_approved)));
    list.push(("dxgi_adapters".into(), Box::new(gpu::dxgi_adapters)));
    list.push(("nvml".into(), Box::new(gpu::nvml)));
    list
}
