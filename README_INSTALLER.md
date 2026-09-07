# Windows 10/11 Installer Branding

The Windows NSIS installer is branded for **MUD Software Company** and Payroll System using the supplied Gorilla Doctors artwork.

Configured in `src-tauri/tauri.conf.json`:
- Installer icon: `installer/payroll-system.ico`
- Header: `installer/header-image.bmp` (150x57)
- Sidebar: `installer/sidebar-image.bmp` (164x314)
- Uninstaller icon/header are also configured.
- Windows WebView2 bootstrapper and minimum runtime are configured for Windows 10/11 compatibility.
- Start Menu folder: `MUD Software Company`

The project also contains `installer/background-installer-image.bmp` as the high-resolution background artwork. Tauri's standard NSIS configuration supports header/sidebar images but does not provide a separate background-image key, so this asset is retained without replacing the stock installer template.
