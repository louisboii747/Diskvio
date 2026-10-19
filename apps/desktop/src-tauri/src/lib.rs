mod menus;
use diskvio_platform::{Capabilities, DiskPlatform, UnavailablePlatform};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    name: &'static str,
    version: &'static str,
    platform: &'static str,
    data_mode: &'static str,
    capabilities: Capabilities,
}

#[tauri::command]
fn get_app_info() -> AppInfo {
    AppInfo {
        name: "Diskvio",
        version: env!("CARGO_PKG_VERSION"),
        platform: diskvio_platform::platform_name(),
        data_mode: "demo",
        capabilities: UnavailablePlatform.capabilities(),
    }
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            menus::install(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_app_info])
        .run(tauri::generate_context!())
        .expect("error while running Diskvio");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn app_info_contract_is_demo_and_default_deny() {
        let value = serde_json::to_value(get_app_info()).unwrap();
        assert_eq!(value["name"], "Diskvio");
        assert_eq!(value["dataMode"], "demo");
        assert_eq!(value["capabilities"]["usbWriting"], false);
        assert_eq!(value["capabilities"]["partitionManagement"], false);
        assert_eq!(value["capabilities"]["discovery"], false);
    }
}
