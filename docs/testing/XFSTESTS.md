# XFSTests Integration

This document explains how to run the xfstests test suite against the Wormhole filesystem and describes the files that enable test execution.

## Overview

XFSTests is a comprehensive test suite for filesystem implementations, originally developed for XFS but now used to test various filesystems including FUSE-based ones. This integration allows testing Wormhole's filesystem implementation against a standardized set of tests.

## Running the Tests

The test suite uses a multi-stage Docker build where the Wormhole binaries are built first, then used by the test image.

### Prerequisites

- Docker and Docker Compose installed
- FUSE support enabled in the kernel
- Sufficient privileges to run Docker containers with `SYS_ADMIN` capability

### Build Instructions

First, build the Wormhole binaries using the `builder` stage and tag the image `wormhole:latest`:

```bash
docker build --target builder -t wormhole:latest .
```

Then, run the xfstests suite using Docker Compose from the `tests/xfstests/` directory:

```bash
cd tests/xfstests
docker compose -f docker-compose.test.yml up --build
```

This will:
1. Reference the previously built `wormhole:latest` image to copy the binaries
2. Build a Docker image containing the xfstests suite and all necessary dependencies
3. Start a container with the required privileges (SYS_ADMIN capability and FUSE device access)
4. Execute the test suite automatically
5. Display test results and summaries

## Test Files

The xfstests integration consists of three main files located in `tests/xfstests/`:

### 1. `run_xfstests_docker.sh`

This is the main test execution script that runs inside the Docker container.

**Responsibilities:**
- Starts the `wormholed` daemon service
- Configures the xfstests environment for FUSE filesystems
- Creates the `local.config` file with test-specific settings
- Executes the xfstests suite with appropriate filters
- Handles cleanup of mounts and daemon processes on exit

**Key Configuration:**
- Uses FUSE filesystem type with `.wormhole` subtype
- Mounts test filesystems at `/mnt/test` and `/mnt/scratch`
- Uses separate pods (`testpod` and `scratchpod`) for test and scratch devices
- Runs the "quick" test group by default
- Excludes problematic tests using `exclude_tests` file

**Exit Behavior:**
The script captures the test exit code and displays a summary of failures before exiting, allowing the container to report test results.

### 2. `mount.fuse.wormhole`

This is a mount helper script that enables mounting Wormhole filesystems using the standard `mount` command with the `subtype=wormhole` option.

**How it works:**
- Called automatically by `/sbin/mount.fuse` when mounting with `subtype=wormhole`
- Parses mount options (pod_name, port) from the mount command
- Cleans up any stale pods or mounts before creating a new one
- Creates a `.global_config.toml` file in the mount point
- Executes `wormhole new` to create and mount the pod
- Logs all operations to `/tmp/mount_helper.log` for debugging

**Mount Options:**
- `pod_name`: Name of the pod to create/mount (default: `default_pod`)
- `port`: Port number for the pod (default: `5000`)
- Standard FUSE options like `allow_other` and `default_permissions` are also supported

**Example Usage:**
```bash
mount -t fuse.wormhole wormhole@test /mnt/test -osubtype=wormhole,pod_name=testpod,port=5000
```

### 3. `xfstests_exclude.txt`

This file contains a list of test cases that should be excluded from the test run.

**Why tests are excluded:**
- **Unimplemented features**: Tests for features not yet implemented in Wormhole (hardlinks, symlinks, ACLs, quotas, etc.)
- **Unsupported operations**: Advanced file operations not supported (O_TMPFILE, fallocate variants, fiemap, etc.)
- **Block device requirements**: Tests that require a real block device rather than a FUSE filesystem
- **Persistence issues**: Tests that fail due to Wormhole's in-memory nature (data lost on remount)
- **Concurrency problems**: Tests that cause deadlocks or hangs with FUSE implementations
- **Known instabilities**: Tests that are unstable or blocking on FUSE filesystems

Each excluded test includes a comment explaining why it's excluded, making it easy to identify tests that could be re-enabled as features are implemented.

## Docker Configuration

### Dockerfile

The `Dockerfile` includes a `test` target that:
- Installs all xfstests dependencies (build tools, libraries, utilities)
- Clones and builds the xfstests-dev repository
- Copies the test scripts and configuration files into the container
- Creates necessary users (`fsgqa`, `123456-fsgqa`) required by xfstests
- Sets up mount points (`/mnt/test`, `/mnt/scratch`)

### docker-compose.test.yml

The Docker Compose configuration:
- Builds the test image using the `test` target
- Grants necessary privileges (`SYS_ADMIN` capability, FUSE device access)
- Runs the test script automatically on container start
- Configures security options to allow FUSE operations

## Test Results

After execution, test results are stored in `/opt/xfstests-dev/results/` inside the container. Key files include:
- `check.log`: Detailed test execution log
- `.config`: Test configuration used
- Individual test result files

The script displays a summary of failures at the end of execution. For detailed analysis, you can access the container's filesystem or mount the results directory.

## Troubleshooting

### Mount Failures
Check `/tmp/mount_helper.log` inside the container for detailed mount operation logs.

### Daemon Issues
Check `/tmp/wormholed.log` for daemon startup and operation logs.

### Test Failures
Review `results/check.log` for detailed failure information. Many failures are expected due to unimplemented features and are documented in `xfstests_exclude.txt`.

## Future Improvements

As Wormhole implements more filesystem features, tests can be removed from `xfstests_exclude.txt` to expand test coverage. Priority areas include:
- Hardlink and symlink support
- Extended attributes and ACLs
- Advanced file operations (fallocate, fiemap)
- Persistence across remounts
- Improved concurrency handling

