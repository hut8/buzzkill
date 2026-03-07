# Buzzkill

Remote ID drone detector. Monitors for drones broadcasting FAA Remote ID (OpenDroneID) over Bluetooth LE and WiFi.

## Features

- Real-time drone detection on BLE and WiFi transports
- GPS receiver support for relative positioning (distance/bearing to detected drones)
- Web dashboard for live monitoring
- PostgreSQL logging of sightings
- Email notifications on drone detection

## Usage

```bash
# BLE only (default)
buzzkill hci0 --port 4200

# BLE + WiFi (requires monitor-mode capable adapter)
buzzkill hci0 --wifi wlan0 --port 4200

# Custom drone expiry timeout (default 60s)
buzzkill hci0 --expiry 120
```

Requires `CAP_NET_ADMIN` and `CAP_NET_RAW` capabilities, or root.

## WiFi Monitor Mode

WiFi Remote ID detection requires a WiFi adapter in monitor mode. Not all WiFi
cards support this. Here is how to determine if yours does and how to enable it.

### Check if Your WiFi Card Supports Monitor Mode

**Step 1: Identify your wireless interface and driver**

```bash
iw dev
```

This lists your wireless interfaces. Note the interface name (e.g., `wlan0`).

To see which driver and chipset your card uses:

```bash
# Show the driver in use
ethtool -i wlan0 | grep driver

# Or check via sysfs
basename $(readlink /sys/class/net/wlan0/device/driver)

# For USB adapters, find the chipset
lsusb

# For PCI/PCIe adapters
lspci -k | grep -A 3 -i network
```

**Step 2: Check supported interface modes**

```bash
iw phy phy0 info | grep -A 10 "Supported interface modes"
```

Replace `phy0` with the correct phy for your interface. To find the phy for your
interface:

```bash
iw dev wlan0 info | grep wiphy
```

If the output includes `monitor`, your card supports monitor mode:

```
Supported interface modes:
         * IBSS
         * managed
         * AP
         * monitor        <-- this is what you need
         * mesh point
```

If `monitor` is **not** listed, your card does not support monitor mode and
cannot be used for WiFi Remote ID scanning with Buzzkill.

**Step 3: Verify it actually works**

Some drivers advertise monitor mode but don't implement it correctly. Test it:

```bash
sudo ip link set wlan0 down
sudo iw dev wlan0 set type monitor
sudo ip link set wlan0 up
```

Then confirm:

```bash
iw dev wlan0 info | grep type
```

Should show `type monitor`. If you get errors, the driver's monitor mode support
is broken or incomplete.

To revert back to managed (normal) mode:

```bash
sudo ip link set wlan0 down
sudo iw dev wlan0 set type managed
sudo ip link set wlan0 up
```

### Chipsets Known to Support Monitor Mode

These chipsets are widely used and have reliable monitor mode support on Linux:

| Chipset Family | Driver | Notes |
|---|---|---|
| Atheros AR9271 | `ath9k_htc` | USB. The gold standard for monitor mode. |
| Ralink RT5370/RT5572 | `rt2800usb` | USB. Cheap and widely available. |
| Mediatek MT7612U | `mt76x2u` | USB. Dual-band 802.11ac. |
| Mediatek MT7921 | `mt7921e` | PCIe. Common in modern laptops. |
| Intel AX200/AX210 | `iwlwifi` | PCIe. Works but may require firmware tweaks. |
| Realtek RTL8812AU | `88XXau` (aircrack-ng) | USB. Requires out-of-tree driver. |

Chipsets that generally **do not** work: most Broadcom (`brcmfmac`/`wl`) and
many Realtek chipsets using in-kernel `rtl8xxxu` or `rtlwifi` drivers.

### Enabling Monitor Mode for Buzzkill

Before starting Buzzkill with `--wifi`, put the interface into monitor mode:

```bash
sudo ip link set wlan0 down
sudo iw dev wlan0 set type monitor
sudo ip link set wlan0 up
```

Then run Buzzkill:

```bash
sudo buzzkill hci0 --wifi wlan0 --port 4200
```

If running as a systemd service, uncomment the `ExecStartPre` lines in
`buzzkill.service` to set monitor mode automatically at startup.

## Building

```bash
cargo build --release
```

## Environment Variables

| Variable | Description |
|---|---|
| `DATABASE_URL` | PostgreSQL connection string. Enables sighting persistence. |
| `SMTP_HOST` | SMTP server for email alerts. |
| `SMTP_USERNAME` | SMTP username. |
| `SMTP_PASSWORD` | SMTP password. |
| `ALERT_TO` | Recipient email for drone alerts. |
| `ALERT_FROM` | Sender email for drone alerts. |

## Deployment

A systemd service file is provided in `buzzkill.service`. See `deploy` for
automated deployment.
