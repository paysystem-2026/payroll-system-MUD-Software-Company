# Windows Installer Branding

Branding assets for Payroll System by MUD Software Company.

- `payroll-system.ico` — Windows installer/uninstaller icon.
- `header-image.bmp` — NSIS installer header, 150x57.
- `sidebar-image.bmp` — NSIS welcome/finish artwork, 164x314.
- `background-installer-image.bmp` — high-resolution installer background artwork retained with the project for branded/custom installer use.
- `x64/` and `x86/` — architecture-labelled copies of the artwork for Windows packaging workflows.
- `gorilla-doctors-source.jpg` — source Gorilla Doctors artwork supplied for the installer branding.
- `installer-branding-reference.png` — approved branding reference/mockup.

The standard Tauri NSIS configuration wires the icon, header and sidebar directly. Tauri's standard NSIS configuration does not expose a separate `backgroundImage` setting; the high-resolution background asset is therefore retained without changing the stock installer template.
