# UDEV Setup for Maschine Jam

This document explains how to set up udev rules to allow non-root access to the Native Instruments Maschine Jam USB device.

## Overview

The Maschine Jam has vendor ID `0x17cc` and product ID `0x1500`. By default, USB devices require root privileges to access. Udev rules allow you to grant specific permissions to your user account.

## Step 1: Identify Your User

First, make sure you know your username:

```bash
whoami
```

## Step 2: Create the Udev Rule

Create a new udev rule file:

```bash
sudo nano /etc/udev/rules.d/99-maschine-jam.rules
```

Add the following content (replace `YOUR_USERNAME` with your actual username):

```
# Native Instruments Maschine Jam
SUBSYSTEM=="usb", ATTRS{idVendor}=="17cc", ATTRS{idProduct}=="1500", MODE="0666", OWNER="YOUR_USERNAME", GROUP="YOUR_USERNAME"
```

## Step 3: Alternative Rule (More Permissive)

If the above doesn't work, you can use a more permissive rule that grants access to all users in the `plugdev` group:

```bash
sudo nano /etc/udev/rules.d/99-maschine-jam.rules
```

Content:
```
# Native Instruments Maschine Jam - permissive access
SUBSYSTEM=="usb", ATTRS{idVendor}=="17cc", ATTRS{idProduct}=="1500", MODE="0666"
```

Then add your user to the plugdev group:
```bash
sudo usermod -a -G plugdev $USER
```

## Step 4: Reload Udev Rules

After creating the rule, reload the udev rules:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

## Step 5: Test the Rule

1. Unplug and replug your Maschine Jam
2. Check if the device is accessible:
   ```bash
   ls -la /dev/bus/usb/*/* | grep 17cc
   ```
   You should see the device with permissions like `crw-rw-rw-` instead of `crw-------`

3. Test with the simple test program (without sudo):
   ```bash
   cargo run --bin simple-maschine-test
   ```

## Step 6: Verify Device Detection

You can also verify the device is detected correctly:

```bash
lsusb | grep 17cc
```

Should show something like:
```
Bus XXX Device XXX: ID 17cc:1500 Native Instruments Maschine Jam
```

## Troubleshooting

### Device Still Not Accessible

1. Check if the rule is loaded:
   ```bash
   sudo udevadm test /sys/class/usb_device/... 2>&1 | grep maschine
   ```

2. Check device attributes:
   ```bash
   sudo udevadm info -a -n /dev/bus/usb/XXX/XXX | grep -E "(idVendor|idProduct)"
   ```

3. Try a more specific rule with additional attributes:
   ```bash
   sudo nano /etc/udev/rules.d/99-maschine-jam.rules
   ```
   
   Content:
   ```
   # Native Instruments Maschine Jam - detailed rule
   SUBSYSTEM=="usb", ATTRS{idVendor}=="17cc", ATTRS{idProduct}=="1500", ATTRS{manufacturer}=="Native Instruments", MODE="0666", OWNER="YOUR_USERNAME"
   ```

### Permission Denied Errors

If you still get permission errors:

1. Check if your user is in the correct groups:
   ```bash
   groups $USER
   ```

2. Try adding your user to the `dialout` group as well:
   ```bash
   sudo usermod -a -G dialout $USER
   ```

3. Log out and log back in for group changes to take effect.

### Device Disconnects

If the device keeps disconnecting:

1. Check if another process is using the device:
   ```bash
   lsof | grep 17cc
   ```

2. Make sure no other applications (like ALSA, JACK, or other MIDI software) are claiming the device.

## Additional Devices

If you have other Native Instruments devices, you can add them to the same rule file:

```
# Native Instruments Maschine Jam
SUBSYSTEM=="usb", ATTRS{idVendor}=="17cc", ATTRS{idProduct}=="1500", MODE="0666", OWNER="YOUR_USERNAME"

# Native Instruments Komplete Kontrol S25 (example)
SUBSYSTEM=="usb", ATTRS{idVendor}=="17cc", ATTRS{idProduct}=="1501", MODE="0666", OWNER="YOUR_USERNAME"
```

## Security Note

The `MODE="0666"` setting gives read/write access to all users. For better security, you can:

1. Use `MODE="0660"` and add only specific users to a group
2. Use `OWNER` and `GROUP` to restrict access to specific users
3. Consider using `TAG+="uaccess"` for automatic user access management

## Verification

After setting up the udev rule, you should be able to run your Rust application without `sudo`:

```bash
cargo run --bin foreigninstruments-event-driven-rusb
```

The application should now detect and read from the Maschine Jam without requiring root privileges. 