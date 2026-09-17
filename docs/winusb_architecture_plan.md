# WinUSB — Architecture, Plan, Milestones & Requirements

## 1. Project Overview

**WinUSB** is a personal, professional-educational Rust systems utility for creating bootable Windows 10/11 USB installation media from Linux/Fedora.

The initial target is:

> Given a Windows ISO and a USB block device, safely produce USB installation media that boots on modern UEFI systems.

The project is intentionally **non-commercial** and focused on learning systems programming, Linux block devices, filesystems, partitioning, ISO images, Windows installation media, and robust Rust architecture.

The project should not attempt to become a general-purpose Ventoy/WoeUSB replacement initially. The first goal is a narrow, understandable, reliable Windows USB writer.

---

# 2. Goals

## Primary Goals

- Create bootable Windows 10/11 USB media from Fedora/Linux.
- Support modern UEFI systems.
- Correctly handle Windows ISOs containing files larger than FAT32's individual-file limit.
- Safely identify removable USB devices.
- Prevent accidental destruction of the wrong block device.
- Separate inspection, planning, validation, execution, and verification.
- Provide useful errors instead of opaque failures.
- Keep the implementation understandable and relatively dependency-light.
- Use Rust to expose and understand the underlying systems concepts rather than hiding everything behind a high-level library.

## Educational Goals

The project should provide hands-on experience with:

- Rust systems programming.
- Linux block devices.
- `/sys` and udev/device discovery.
- Filesystem concepts.
- GPT partition tables.
- FAT32 and potentially NTFS.
- ISO 9660/UDF.
- UEFI boot structure.
- Windows WIM/ESD installation images.
- WIM splitting.
- File extraction.
- Mounting/unmounting filesystems.
- Device I/O.
- Synchronization and flushing.
- Verification.
- Safe destructive-operation design.
- Error modeling.
- CLI architecture.

---

# 3. Non-Goals — Initial Version

The first version should NOT attempt to:

- Replace Ventoy completely.
- Support arbitrary Linux distributions.
- Support every possible Windows installation-media variant.
- Implement a complete ISO filesystem parser from scratch unless useful educationally.
- Support BIOS/legacy boot initially.
- Support macOS or Windows initially.
- Provide a graphical interface.
- Implement persistent live environments.
- Implement multi-ISO booting.
- Automatically download Windows ISOs.
- Circumvent Windows activation/licensing.
- Modify Windows installations beyond what is necessary to create installation media.

These may become future projects/features, but they should not complicate the MVP.

---

# 4. High-Level Architecture

The application follows a pipeline:

    Discovery
        ↓
    Inspection
        ↓
    Planning
        ↓
    Validation
        ↓
    User Confirmation
        ↓
    Execution
        ↓
    Verification

The critical architectural rule is:

> Planning must be separate from destructive execution.

The program should be able to calculate and display exactly what it intends to do before it modifies the target device.

Conceptually:

                    ┌──────────────┐
                    │     CLI      │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │   Inspect    │
                    └──────┬───────┘
                           │
          ┌────────────────┴────────────────┐
          │                                 │
          ▼                                 ▼
    ┌───────────┐                     ┌─────────────┐
    │ WindowsISO│                     │ BlockDevice │
    └─────┬─────┘                     └──────┬──────┘
          │                                  │
          └────────────────┬─────────────────┘
                           ▼
                    ┌────────────┐
                    │   Planner  │
                    └──────┬─────┘
                           │
                           ▼
                      WritePlan
                           │
                           ▼
                    ┌────────────┐
                    │  Validator │
                    └──────┬─────┘
                           │
                           ▼
                    ┌────────────┐
                    │  Executor  │
                    └──────┬─────┘
                           │
                           ▼
                    ┌────────────┐
                    │  Verifier  │
                    └────────────┘

---

# 5. Initial Project Structure

Start as a single Cargo package.

    winusb/
    ├── Cargo.toml
    ├── Cargo.lock
    ├── README.md
    ├── LICENSE
    ├── .gitignore
    │
    ├── src/
    │   ├── main.rs
    │   ├── cli.rs
    │   ├── error.rs
    │   ├── iso.rs
    │   ├── device.rs
    │   ├── partition.rs
    │   ├── filesystem.rs
    │   ├── writer.rs
    │   └── verifier.rs
    │
    └── tests/
        └── ...

Do not create a multi-crate workspace immediately.

Split into crates only when the boundaries become meaningful.

A future structure could become:

    winusb/
    ├── crates/
    │   ├── winusb/
    │   ├── winusb-core/
    │   └── winusb-platform/
    └── ...

Possible future responsibility split:

- `winusb`: CLI/application layer.
- `winusb-core`: platform-independent planning and Windows-media logic.
- `winusb-platform`: Linux-specific device, partition, mount, and filesystem operations.

---

# 6. Architectural Layers

## 6.1 CLI Layer

Responsible for:

- Argument parsing.
- Command dispatch.
- User interaction.
- Confirmation prompts.
- Human-readable output.
- Exit codes.

Initial commands:

    winusb inspect <ISO>
    winusb devices
    winusb plan <ISO> <DEVICE>
    winusb write <ISO> <DEVICE>
    winusb verify <DEVICE>

The CLI should not contain low-level device manipulation.

---

## 6.2 ISO/Image Layer

Responsible for representing and inspecting an ISO.

Initial conceptual type:

    struct WindowsIso {
        path: PathBuf,
    }

Possible metadata:

    struct ImageMetadata {
        size: u64,
        filesystem: ImageFilesystem,
        architecture: Architecture,
        boot_mode: BootMode,
        windows_version: Option<String>,
        install_image: InstallImageInfo,
    }

The rest of the application should not need to know the details of ISO 9660/UDF implementation.

Responsibilities:

- Open ISO.
- Inspect filesystem.
- Enumerate files.
- Locate Windows boot files.
- Locate `boot.wim`.
- Locate `install.wim` or `install.esd`.
- Determine relevant file sizes.
- Identify architecture where possible.
- Detect unsupported media.

---

## 6.3 Device Layer

Represents physical block devices.

Initial conceptual type:

    struct BlockDevice {
        path: PathBuf,
        model: Option<String>,
        size: u64,
        removable: bool,
    }

Linux discovery should eventually use appropriate kernel interfaces such as:

- `/sys/class/block`
- udev where useful
- block-device metadata
- ioctl interfaces where required

The device layer should distinguish:

- Physical/removable USB devices.
- Internal disks.
- Partitions.
- Mounted devices.

A destructive command should strongly prefer a whole removable block device such as:

    /dev/sdb

rather than silently operating on a partition such as:

    /dev/sdb1

---

# 7. Partition Layer

Responsible for describing and creating the target partition layout.

Initial target:

- GPT partition table.
- UEFI-compatible layout.
- FAT32 boot/system partition where appropriate.
- Additional filesystem/partition strategy when required by Windows media constraints.

Conceptual types:

    struct PartitionTable {
        kind: PartitionTableKind,
        partitions: Vec<PartitionSpec>,
    }

    struct PartitionSpec {
        number: u32,
        size: u64,
        filesystem: FilesystemKind,
        role: PartitionRole,
    }

The partition planner must be able to produce a plan without modifying the disk.

---

# 8. Filesystem Layer

Responsible for filesystem creation and file operations.

Initial considerations:

- FAT32.
- NTFS if required by the chosen Windows-media strategy.

Responsibilities:

- Create filesystem.
- Mount filesystem.
- Copy/extract files.
- Preserve relevant paths.
- Unmount cleanly.
- Flush pending writes.

The implementation should distinguish filesystem logic from partition-table logic.

---

# 9. Windows Media Layer

Windows installation media has constraints that make a naive ISO-to-USB copy unreliable.

Important issue:

FAT32 has an individual-file size limit of approximately 4 GiB.

A Windows ISO may contain:

    sources/install.wim

that exceeds that limit.

Therefore:

    ISO → FAT32 → copy everything

is not sufficient as a general strategy.

The project must support a Windows-specific media strategy.

Initial strategy candidate:

    install.wim
        ↓
    split into:
        install.swm
        install2.swm
        install3.swm
        ...

Windows Setup can consume split WIM images.

Alternative future strategy:

    GPT
      ├── EFI/FAT32 boot partition
      └── NTFS Windows data partition

The exact strategy should be finalized during the implementation milestone after testing representative Windows 10/11 ISOs and firmware behavior.

---

# 10. WritePlan

The central safety abstraction is a write plan.

Conceptually:

    struct WritePlan {
        source: WindowsIso,
        target: BlockDevice,
        partition_table: PartitionTable,
        partitions: Vec<PartitionSpec>,
        operations: Vec<WriteOperation>,
    }

A plan describes what will happen but does not execute it.

Example conceptual output:

    Target: /dev/sdb
    Model: Kingston DataTraveler
    Size: 32 GiB

    Existing partition table:
        GPT

    Planned operation:
        destroy existing partition table

    New layout:
        GPT
        partition 1
            FAT32
            boot/system
            32 GiB

    Windows media:
        architecture: x86_64
        install image: install.wim
        install image size: 5.8 GiB
        WIM splitting: required

---

# 11. Validation

Before execution, validate:

## Source

- ISO exists.
- ISO is readable.
- ISO appears to contain Windows installation media.
- Required boot files exist.
- Required installation image exists.
- Media format is supported.

## Target

- Device exists.
- Device is a block device.
- Device is not the source filesystem.
- Device is large enough.
- Device is writable.
- Device is not an obviously dangerous internal system disk unless the user explicitly confirms an override in a future design.
- Required partitions are not mounted before destruction.

## Plan

- Partition layout is valid.
- Filesystem constraints are satisfied.
- Large files have a valid handling strategy.
- Expected final capacity is sufficient.

Validation should happen before any destructive action.

---

# 12. Destructive Operation Safety

Writing a Windows USB is inherently destructive.

The safety model should be explicit.

Before writing:

    Windows ISO:
      Win11.iso
      6.4 GiB

    Target:
      /dev/sdb
      Kingston DataTraveler
      32 GiB
      removable: yes

    WARNING:
      All existing data on /dev/sdb will be destroyed.

    Type the device name to continue:
      > sdb

The program should not accept a generic `y` confirmation alone for the initial implementation.

The user should confirm the actual target identifier.

Additional future safeguards:

- Require `--yes` for scripting.
- Require `--force` for non-removable devices.
- Detect mounted partitions.
- Refuse to operate on the system/root disk by default.
- Show stable device identifiers such as model/serial where available.
- Revalidate the target immediately before destruction.
- Refuse if the device identity changed between planning and execution.

---

# 13. Error Architecture

Errors should be meaningful and layered.

Conceptual error enum:

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("failed to inspect ISO: {0}")]
        Iso(#[source] std::io::Error),

        #[error("failed to access block device: {0}")]
        Device(#[source] std::io::Error),

        #[error("target device is too small")]
        DeviceTooSmall,

        #[error("ISO contains a file larger than the target filesystem limit")]
        FileTooLarge,

        #[error("unsupported Windows installation media")]
        UnsupportedWindowsMedia,

        #[error("target device has mounted partitions")]
        DeviceBusy,

        #[error("operation cancelled")]
        Cancelled,

        #[error("verification failed")]
        VerificationFailed,
    }

The CLI should transform low-level errors into useful user-facing messages.

For example:

Bad:

    failed: error 22

Better:

    Cannot create the target filesystem:
    the target device does not support the requested operation.

Best where available:

    Cannot copy sources/install.wim:
    the file is 5.8 GiB, but FAT32 cannot store individual files
    larger than approximately 4 GiB.

---

# 14. Dependency Philosophy

Keep the dependency footprint small.

Initial candidates:

    clap
    thiserror
    anyhow
    tracing
    tracing-subscriber
    indicatif

Potential roles:

- `clap`: CLI parsing.
- `thiserror`: library/domain errors.
- `anyhow`: application-level error context where appropriate.
- `tracing`: diagnostics/logging.
- `tracing-subscriber`: logging output.
- `indicatif`: progress display.

Specialized dependencies for:

- ISO parsing.
- GPT/partition manipulation.
- FAT32/NTFS.
- WIM processing.

should be introduced only when the corresponding subsystem is implemented.

Do not add a large high-level "Windows USB creator" dependency that hides the systems concepts being learned.

---

# 15. Core Design Principles

## 15.1 Separate Planning From Execution

Never combine:

    detect
    partition
    format
    copy
    verify

inside one opaque function.

Prefer:

    discover()
        ↓
    inspect()
        ↓
    plan()
        ↓
    validate()
        ↓
    confirm()
        ↓
    execute()
        ↓
    verify()

---

## 15.2 Keep Destructive Code Localized

The code capable of destroying a block device should have a very small surface area.

Ideally, a reader of the code can immediately answer:

> Where can this program erase a disk?

---

## 15.3 Prefer Concrete Types Initially

Do not start with dozens of traits.

Start with concrete domain types:

    WindowsIso
    BlockDevice
    PartitionSpec
    WritePlan

Introduce traits only when there is a real boundary, such as:

    ImageReader
    DeviceProvider
    Filesystem
    Writer

---

## 15.4 Make Dry-Run/Planning a First-Class Capability

A user should eventually be able to run:

    winusb plan Win11.iso /dev/sdb

and see exactly what would happen without modifying the device.

---

## 15.5 Verification Is Part of the Operation

A successful write call does not necessarily mean the USB is usable.

Verification should eventually check:

- Partition table.
- Expected files.
- EFI boot files.
- File sizes.
- WIM/SWM files.
- Filesystem accessibility.
- Flush completion.

---

# 16. Milestones

## Milestone 0 — Project Bootstrap

### Objective

Create the Rust project and CLI skeleton.

### Requirements

- Rust stable toolchain.
- Cargo.
- Fedora/Linux development environment.
- Git repository.
- Basic CLI.
- Error handling.
- Logging.

### Commands

    winusb --help
    winusb inspect <ISO>
    winusb devices
    winusb plan <ISO> <DEVICE>
    winusb write <ISO> <DEVICE>
    winusb verify <DEVICE>

At this stage:

`write` must not modify anything.

### Completion Criteria

- Project compiles.
- CLI commands parse.
- Errors are human-readable.
- `--help` is useful.
- No destructive operations exist yet.

---

# 17. Milestone 1 — ISO Inspector

### Objective

Understand and inspect Windows ISO contents.

### Requirements

- Open ISO.
- Determine ISO size.
- Read ISO filesystem metadata.
- Enumerate files.
- Locate:
    - `boot/`
    - `efi/`
    - `EFI/BOOT/`
    - `bootmgr`
    - `bootmgr.efi`
    - `sources/boot.wim`
    - `sources/install.wim`
    - `sources/install.esd`

### Command

    winusb inspect Win11.iso

### Expected information

    Image
      size: 6.4 GiB
      filesystem: ISO/UDF

    Windows
      architecture: x86_64
      UEFI boot: detected

    Installation image
      type: WIM
      size: 5.8 GiB
      FAT32-compatible: no

### Completion Criteria

The program can determine whether a supplied ISO looks like supported Windows installation media.

---

# 18. Milestone 2 — Linux USB Discovery

### Objective

Discover block devices safely.

### Command

    winusb devices

### Example

    DEVICE       MODEL                  SIZE       REMOVABLE
    /dev/sda     Samsung SSD            1.0 TB     no
    /dev/sdb     Kingston DataTraveler   32 GB      yes

### Requirements

- Enumerate block devices.
- Identify removable devices.
- Read device size.
- Read model/vendor where possible.
- Distinguish disks from partitions.
- Detect mounted partitions.
- Prefer stable device metadata.

### Completion Criteria

The program can reliably identify the intended USB device without requiring manual `/dev/sdX` guessing.

---

# 19. Milestone 3 — Partition Planning

### Objective

Generate a target USB layout without changing the device.

### Command

    winusb plan Win11.iso /dev/sdb

### Requirements

- Read current device state.
- Detect existing partition table.
- Determine required Windows-media strategy.
- Generate GPT layout.
- Determine filesystem requirements.
- Determine WIM splitting requirement.
- Generate ordered write operations.

### Completion Criteria

`plan` produces a complete human-readable description of the intended operation.

No disk modifications occur.

---

# 20. Milestone 4 — Target Validation

### Objective

Prevent invalid or dangerous writes.

### Requirements

Validate:

- ISO readability.
- Windows-media structure.
- Device existence.
- Device writability.
- Device capacity.
- Device identity.
- Removability.
- Mounted partitions.
- Partition strategy.
- Filesystem constraints.
- WIM handling strategy.

### Completion Criteria

Invalid targets fail before destructive execution begins.

---

# 21. Milestone 5 — Partition and Filesystem Creation

### Objective

Create the target disk layout.

### Operations

    block device
        ↓
    GPT
        ↓
    partitions
        ↓
    filesystem creation
        ↓
    mount

### Requirements

- Create GPT.
- Create required partitions.
- Format required filesystems.
- Mount them.
- Cleanly unmount them.
- Flush changes.

### Safety

This is the first milestone where the application modifies a physical device.

Testing should initially use disposable USB drives.

### Completion Criteria

The application can create the planned empty target layout reliably.

---

# 22. Milestone 6 — Windows File Extraction

### Objective

Copy Windows installation media onto the target.

### Requirements

- Extract/copy all required files.
- Preserve directory structure.
- Handle large files correctly.
- Display progress.
- Handle interrupted operations cleanly where possible.
- Flush writes.
- Avoid silently skipping files.

### Special case

If using FAT32:

    install.wim > ~4 GiB

must trigger a valid Windows-compatible strategy.

Initial candidate:

    install.wim
        ↓
    split WIM
        ↓
    install.swm
    install2.swm
    install3.swm
    ...

### Completion Criteria

A representative Windows ISO can be transformed into complete installation media.

---

# 23. Milestone 7 — Boot Verification

### Objective

Verify that the generated media has the required boot structure.

### Command

    winusb verify /dev/sdb

### Requirements

Verify:

- GPT exists.
- Expected partitions exist.
- Expected filesystem exists.
- EFI boot files exist.
- Windows installation files exist.
- WIM/SWM structure is correct.
- Filesystem can be read after unmount/remount where practical.

### Completion Criteria

The verifier can distinguish between an incomplete USB and a structurally valid one.

---

# 24. Milestone 8 — Real Hardware Validation

### Objective

Test generated USB media on actual machines.

Test matrix should include:

- At least one UEFI desktop.
- At least one UEFI laptop.
- Multiple USB drives if available.
- Windows 10 ISO.
- Windows 11 ISO.
- ISO with `install.wim` under 4 GiB if available.
- ISO with `install.wim` over 4 GiB.
- Different USB capacities.

Record:

- Firmware detects USB.
- Windows Boot Manager starts.
- Windows Setup starts.
- Installation image is detected.
- Installation proceeds beyond media selection.

The test results should be documented rather than treated as assumptions.

---

# 25. Milestone 9 — CLI and UX Hardening

### Objective

Make the utility pleasant and safe to use.

Potential features:

    winusb devices
    winusb inspect image.iso
    winusb plan image.iso /dev/sdb
    winusb write image.iso /dev/sdb
    winusb verify /dev/sdb

Additional options:

    --yes
    --dry-run
    --verbose
    --quiet
    --json
    --force

Potential progress display:

    Inspecting ISO...       done
    Validating target...    done
    Creating GPT...        done
    Formatting FAT32...    done
    Extracting files...    63%
    Verifying media...     done

---

# 26. Milestone 10 — Architecture Refactor

Only after the system works should we evaluate splitting the project into crates.

Possible final structure:

    winusb/
    ├── crates/
    │   ├── winusb/
    │   │   └── src/
    │   │       ├── main.rs
    │   │       ├── cli.rs
    │   │       └── commands/
    │   │           ├── inspect.rs
    │   │           ├── devices.rs
    │   │           ├── plan.rs
    │   │           ├── write.rs
    │   │           └── verify.rs
    │   │
    │   ├── winusb-core/
    │   │   └── src/
    │   │       ├── lib.rs
    │   │       ├── iso.rs
    │   │       ├── windows.rs
    │   │       ├── plan.rs
    │   │       ├── partition.rs
    │   │       ├── filesystem.rs
    │   │       ├── writer.rs
    │   │       └── error.rs
    │   │
    │   └── winusb-platform/
    │       └── src/
    │           ├── lib.rs
    │           └── linux.rs
    │
    └── tests/

Possible dependency direction:

    CLI
      ↓
    Core
      ↓
    Platform

The core should contain as little Linux-specific code as practical.

---

# 27. Functional Requirements

## FR-001 — ISO Input

The application must accept a local Windows ISO path.

## FR-002 — ISO Inspection

The application must inspect the ISO before writing.

## FR-003 — Windows Detection

The application must determine whether the ISO contains recognizable Windows installation media.

## FR-004 — Device Discovery

The application must enumerate available block devices on Linux.

## FR-005 — Removable Device Detection

The application must identify removable devices.

## FR-006 — Device Capacity

The application must determine target capacity.

## FR-007 — Planning

The application must generate a write plan without modifying the target.

## FR-008 — Validation

The application must validate source, target, and plan before execution.

## FR-009 — Destructive Confirmation

The application must explicitly confirm destructive operations.

## FR-010 — Partitioning

The application must create the planned partition layout.

## FR-011 — Filesystem Creation

The application must create required filesystems.

## FR-012 — Windows File Installation

The application must copy/extract required Windows installation files.

## FR-013 — Large WIM Handling

The application must handle installation images that exceed the target filesystem's individual-file limit.

## FR-014 — Verification

The application must verify the resulting media.

## FR-015 — Errors

The application must report actionable errors.

---

# 28. Non-Functional Requirements

## NFR-001 — Safety

The application must minimize the possibility of destroying an unintended disk.

## NFR-002 — Observability

Operations should be visible through structured logging and useful progress output.

## NFR-003 — Deterministic Planning

The same source and target state should produce an explainable plan.

## NFR-004 — Testability

Planning and validation logic should be testable without physical USB hardware.

## NFR-005 — Maintainability

Low-level platform code should not leak throughout the application.

## NFR-006 — Dependency Discipline

Dependencies should be introduced only when they solve a concrete problem.

## NFR-007 — Recoverability

Where practical, failures should leave the device in a known state and provide enough information to diagnose the failure.

## NFR-008 — Performance

Large Windows files should be copied efficiently without unnecessary buffering or whole-file memory allocation.

---

# 29. Testing Strategy

Testing should exist at multiple levels.

## Unit Tests

Test:

- ISO metadata parsing.
- Windows-media detection.
- File-size decisions.
- WIM splitting decisions.
- Device validation.
- Capacity calculations.
- Partition planning.
- Write-plan generation.

These should not require a physical USB device.

## Integration Tests

Test:

- ISO inspection.
- Filesystem creation.
- Extraction.
- Mount/unmount.
- Verification.

These can use temporary files and loopback block devices where appropriate.

## Hardware Tests

Use disposable physical USB drives.

Never use the main development disk.

---

# 30. Security and Safety Considerations

The program performs privileged and destructive operations.

Linux permissions will likely require root privileges for some operations.

Do not solve this by running the entire application as root unnecessarily.

Prefer:

    normal user
        ↓
    inspect / plan / validate
        ↓
    privileged operation only when needed

Potential future design:

- Separate privileged operations from normal CLI logic.
- Minimize the privileged code path.
- Revalidate device identity immediately before destructive operations.

---

# 31. Initial Requisites

## Development Environment

- Fedora Linux.
- Rust stable.
- Cargo.
- Git.
- A disposable USB drive.
- A Windows 10/11 ISO for testing.
- Access to at least one UEFI-capable test machine.

## Knowledge Prerequisites

Useful but not required:

- Basic Rust.
- Ownership/borrowing.
- `Result` / `Option`.
- Traits.
- Iterators.
- Filesystem APIs.
- Linux shell.
- Basic disk/partition concepts.

The project itself is intended to teach the deeper systems concepts.

---

# 32. Initial Research Questions

Before implementing destructive operations, answer:

1. What filesystem(s) are present in current Windows 10/11 ISOs?
2. What boot files are required for UEFI boot?
3. What exactly does Windows Setup require from the USB layout?
4. When is `install.wim` larger than 4 GiB?
5. How does Windows Setup consume split `.swm` files?
6. Can a FAT32-only layout support the target Windows media?
7. When is an NTFS data partition necessary?
8. How does UEFI firmware locate the Windows bootloader?
9. Which partition type GUIDs are appropriate?
10. What Linux APIs should be used for block-device discovery?
11. How should mounted partitions be detected?
12. How should filesystem creation be performed?
13. How should device writes be flushed and verified?
14. What happens when the USB is unplugged during writing?
15. How should partially-written media be handled?
16. What Windows ISO variations need to be rejected?

These questions should be answered with experiments and documentation before locking the final implementation strategy.

---

# 33. First Implementation Order

The recommended implementation order is:

    1. Cargo project
    2. CLI
    3. Error model
    4. ISO inspection
    5. Linux device discovery
    6. Windows-media analysis
    7. WritePlan
    8. Validation
    9. Partition planning
    10. Filesystem strategy
    11. Actual partition creation
    12. Filesystem creation
    13. File extraction
    14. WIM splitting
    15. Verification
    16. Real hardware testing
    17. UX hardening
    18. Architecture refactor

Do not reverse this order by starting with raw disk writes.

---

# 34. Definition of MVP

The MVP is complete when:

1. A Windows 10/11 ISO can be inspected.
2. A removable USB device can be safely identified.
3. The program can produce a write plan.
4. The plan is validated before execution.
5. The user explicitly confirms the destructive operation.
6. The USB is partitioned and formatted according to the selected strategy.
7. Windows installation files are copied correctly.
8. Large installation images are handled correctly.
9. The resulting USB contains the required UEFI boot structure.
10. Verification succeeds.
11. The USB successfully boots into Windows Setup on representative UEFI hardware.

---

# 35. Long-Term Possibilities

Only after the MVP is reliable:

- Legacy BIOS support.
- More Windows ISO variants.
- NTFS/FAT32 hybrid strategies.
- Better WIM/ESD inspection.
- Automatic strategy selection.
- Multiple USB targets.
- Resume support.
- Parallel file extraction.
- Checksums.
- JSON output.
- Machine-readable plans.
- Library API.
- Additional operating systems.
- A GUI.
- ISO download/integrity verification.
- Boot-media diagnostics.

These should remain separate from the initial MVP.

---

# 36. Project Philosophy

WinUSB should remain a small systems utility rather than becoming an abstraction-heavy framework.

The guiding principle is:

> Understand the layers, model them explicitly, automate only after understanding them.

The project should make the following pipeline visible:

    Windows ISO
        ↓
    ISO filesystem
        ↓
    Windows media analysis
        ↓
    Target-device discovery
        ↓
    Partition strategy
        ↓
    Filesystem strategy
        ↓
    File transformation/extraction
        ↓
    USB installation media
        ↓
    UEFI boot
        ↓
    Verification

The first implementation should favor correctness, safety, observability, and learning over feature count.
