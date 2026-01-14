#!/bin/bash
set -e

# --- Configuration ---
WORMHOLE_DAEMON="/bin/wormholed"
PID_FILE="/test/fuse.pid"
MOUNT_HELPER_LOG="/tmp/mount_helper.log"

# --- Cleanup function ---
cleanup() {
    EXIT_CODE=$?
    echo "--- Cleanup (Exit Code: $EXIT_CODE) ---"
    
    # Si échec, on dump le log de montage pour comprendre pourquoi wormhole a refusé de monter
    if [ $EXIT_CODE -ne 0 ] && [ -f "$MOUNT_HELPER_LOG" ]; then
        echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
        echo "DUMPING MOUNT HELPER LOG (Why did mount fail?):"
        cat "$MOUNT_HELPER_LOG"
        echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!"
    fi

    if [ -f "$PID_FILE" ]; then
        echo "Stopping the wormholed service (PID $(cat "$PID_FILE"))..."
        kill "$(cat "$PID_FILE")" || echo "Daemon already stopped."
        rm -f "$PID_FILE"
        sleep 1
    fi
    if mount | grep -q -- "/mnt/test"; then
        fusermount -u /mnt/test || echo "Cleanup: /mnt/test unmount failed."
    fi
    if mount | grep -q -- "/mnt/scratch"; then
        fusermount -u /mnt/scratch || echo "Cleanup: /mnt/scratch unmount failed."
    fi
}
trap cleanup EXIT ERR

# --- Preparation ---
echo "--- Preparation of the test environment ---"

mkdir -p /opt/xfstests-dev/results

chmod 777 /mnt/test /mnt/scratch
chmod -R 777 /opt/xfstests-dev/results

rm -f "$MOUNT_HELPER_LOG"


echo "Starting the wormholed service..."
"$WORMHOLE_DAEMON" > /tmp/wormholed.log 2>&1 &
echo "$!" > "$PID_FILE"
echo "Daemon started with PID $(cat "$PID_FILE")"
sleep 2

# --- Execution of the Tests ---
echo "--- Execution of xfstests ---"
cd /opt/xfstests-dev

# Patch common/rc
sed -i 's|fuse.ceph-fuse/ceph-fuse/|fuse.ceph-fuse/ceph-fuse/;s/fuse.wormhole/fuse/|' common/rc
echo "Patched common/rc to support fuse.wormhole"

touch /tmp/test_dev_file
touch /tmp/scratch_dev_file

# --- Create local.config ---
echo "Creating local.config file..."
cat << EOF > local.config
export FSTYP=fuse
export FUSE_SUBTYP=".wormhole"
export TEST_DEV="wormhole@test"
export SCRATCH_DEV="wormhole@scratch"
export TEST_DIR="/mnt/test"
export SCRATCH_MNT="/mnt/scratch"
export TEST_FS_MOUNT_OPTS="-osubtype=wormhole,pod_name=testpod,port=5000,allow_other,default_permissions"
export MOUNT_OPTIONS="-osubtype=wormhole,pod_name=scratchpod,port=5001,allow_other,default_permissions"
export RESULT_BASE="\$PWD/results"
EOF

# --- Launch the test ---
# Le fichier 'exclude_tests' a été copié ici par le Dockerfile
TEST_CMD="./check -fuse -E exclude_tests -g quick"

echo "-----------------------------------------------------"
echo "Running command: $TEST_CMD"
echo "-----------------------------------------------------"

rm -f "results/.config" /tmp/mount_helper.log

# 1. Disable 'set -e' temporarily
set +e

# 2. Execute the command:
$TEST_CMD

# 3. Capture exit code
EXIT_CODE=$?

# 4. Reactivate 'set -e'
set -e

echo "-----------------------------------------------------"
echo "Tests finished."
echo "Check results/ directory for details."
echo "-----------------------------------------------------"

if [ -f "results/check.log" ]; then
    echo "--- Summary of Failures ---"
    grep "Failures:" results/check.log || echo "No global failure summary found."
    grep -E "^generic/[0-9]+.*FAIL" results/check.log || true
fi