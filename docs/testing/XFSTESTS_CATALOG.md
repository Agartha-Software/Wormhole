# XFSTests Documentation for Wormhole

This document details the tests from the **xfstests** suite used to validate the Wormhole file system. We primarily use the `quick` group for continuous integration.

## Understanding Groups and Types

Tests in xfstests are organized by **groups**. A single test can belong to multiple groups.

### Main Groups

* **quick** : Tests that run quickly (typically a few seconds per test). This is our primary target for CI.
* **auto** : The standard group for automated tests (often includes `quick`).
* **dangerous** : Tests that can crash the kernel (generally avoided).

### Feature Types (Categories)

Tests are often tagged with types describing what they test:

* **rw (Read/Write)** : Data read and write operations, data integrity verification.
* **metadata** : Inode manipulation, file creation/deletion, renaming.
* **perms** : POSIX permissions, `chmod`, `chown`, `suid`, `sgid`.
* **attr** : Extended attributes (xattr).
* **ioctl** : System calls specific via `ioctl`.
* **mmap** : Memory mapped files (memory mapped I/O).
* **aio** : Asynchronous input/output operations.

## Excluded Tests

Some tests from the `quick` group are excluded because Wormhole does not yet support certain features. These exclusions are defined in `tests/xfstests/xfstests_exclude.txt`.

The main currently unsupported categories are:

* **Hardlinks & Symlinks** (Hard and symbolic links)
* **ACLs** (Access Control Lists)
* **Advanced File Ops** (fallocate, fiemap, sparse files)
* **Persistence** (Since Wormhole is in-memory for development, tests requiring a remount fail)

---

## Test List (Group 'quick')

> **Note:** This list is generated dynamically from the test image. To update it, run the following command from the project root:
>
> ```bash
> docker compose -f tests/xfstests/docker-compose.test.yml run --rm --entrypoint /tests/generate_test_list.sh xfstests
> ```

You can find it [here](/docs/testing/xfstests_full_list.md)

### Example of Typical Tests (Non-exhaustive List)

#### Category: rw

| Test ID | Description |
|---|---|
| **generic/001** | Random file copies and unlink |
| **generic/013** | fsstress (Stress test filesystem operations) |
| **generic/129** | Loop read/write verification |

#### Category: metadata

| Test ID | Description |
|---|---|
| **generic/007** | Rename, link, unlink stability |
| **generic/028** | Race conditions in path lookup |

#### Category: perms

| Test ID | Description |
|---|---|
| **generic/128** | Permission checks for basic operations |
| **generic/306** | chmod/chown behavior verification |
