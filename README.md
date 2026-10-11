# winusb

`winusb` is a Rust learning project for preparing a bootable Windows installation USB on Linux.

It reads a Windows ISO directly as UDF media; it does not mount the ISO. The current implementation analyses the Windows media, creates a FAT32-friendly payload, splits a large `install.wim` into `.swm` files, writes that payload to a removable GPT/UEFI USB drive, and verifies the resulting payload.

## Status

The core preparation and writing path has been tested through Windows Setup boot:

- Direct UDF ISO inspection and Windows-media discovery.
- `boot.wim` / `install.wim` discovery and install-image metadata reading.
- FAT32 plan generation and `install.wim` splitting when it exceeds FAT32's file-size limit.
- Staging, writing, synchronisation, and read-only USB payload verification.
- GPT with one FAT32 EFI System Partition for modern UEFI computers.

`--bypass-checks` is experimental. It patches `sources/boot.wim` image 2 with the Windows 11 TPM, Secure Boot, and RAM LabConfig values and an unattended fallback. The patch is verified before the staged WIM replaces the original. It is **not yet considered a complete replacement for Rufus's Windows 11 compatibility bypasses**.

## Development dependencies

The project currently uses local forks configured with absolute paths in `Cargo.toml`:

- `hadris-optical` / `hadris-udf` for UDF and ISO reading.
- `wim-parser` for WIM metadata.

Those paths must exist before Cargo can build the project. This is intentional for the current learning setup, but makes the repository non-portable until dependency configuration is redesigned.

On Fedora, install the external tools used by the preparation and write paths:

```bash
sudo dnf install wimlib perl-hivex parted dosfstools util-linux systemd
```

`wimlib-imagex` splits and updates WIM files. `hivexregedit` is required only by `prepare --bypass-checks`.

Build with:

```bash
cargo build
```

## Workflow

Use the binary built by Cargo below. Replace the example paths with your ISO, staging directory, and removable device.

### 1. Inspect the ISO

```bash
./target/debug/winusb inspect /path/to/windows.iso
```

This reports discovered Windows boot files, WIM files, install editions, and ISO contents.

### 2. Inspect candidate devices and the plan

```bash
./target/debug/winusb devices
./target/debug/winusb plan /path/to/windows.iso /dev/sdX
```

`plan` is read-only. It validates that the target is removable and sufficiently large.

### 3. Prepare a staging payload

The destination directory must be new or empty.

```bash
./target/debug/winusb prepare \
  /path/to/windows.iso \
  /var/tmp/winusb-payload \
  /var/tmp
```

For the experimental Windows 11 compatibility customization:

```bash
./target/debug/winusb prepare \
  /path/to/windows.iso \
  /var/tmp/winusb-payload \
  /var/tmp \
  --bypass-checks
```

Preparation copies the ISO tree, excludes an oversized `sources/install.wim`, then uses `wimlib-imagex split` to create `sources/install.swm`, `install2.swm`, and further parts as necessary.

### 4. Write the prepared payload

```bash
sudo ./target/debug/winusb write \
  /path/to/windows.iso \
  /var/tmp/winusb-payload \
  /dev/sdX
```

This operation is destructive. It creates a GPT partition table, formats `/dev/sdX1` as FAT32, and asks you to type the kernel device name before continuing.

### 5. Verify the USB

```bash
sudo ./target/debug/winusb verify /dev/sdX
```

Verification mounts the written partition read-only and confirms the required Windows payload files exist.

## Safety boundaries

- `write` destroys all existing data on the selected device.
- Always identify the USB with `devices` before using `/dev/sdX`.
- Do not target an internal disk.
- The project currently targets Linux tools and a modern UEFI/FAT32 USB layout.

## Useful commands

```bash
./target/debug/winusb requirements
./target/debug/winusb stage /path/to/windows.iso /var/tmp/winusb-stage
./target/debug/winusb split-stage /path/to/windows.iso /var/tmp/winusb-payload /var/tmp
```

`stage` and `split-stage` are lower-level development commands. In normal use, prefer `prepare`, then `write`, then `verify`.
