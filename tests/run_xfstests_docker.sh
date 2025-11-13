#!/bin/bash
# This script is executed as root in the Docker container.
set -e

# --- Configuration ---
WORMHOLE_DAEMON="/bin/wormholed"
WORMHOLE_CLI="/bin/wormhole"
PID_FILE="/test/fuse.pid"

# Utilise les variables de l'environnement (docker-compose)
: "${TEST_DIR:=/mnt/wormhole-test}"
: "${SCRATCH_MNT:=${TEST_DIR}/scratch}"
: "${FSTYP:=fuse}"


# --- Cleanup function ---
cleanup() {
    echo "--- Cleanup ---"
    
    if [ -f "$PID_FILE" ]; then
        echo "Stopping the wormholed service (PID $(cat "$PID_FILE"))..."
        kill "$(cat "$PID_FILE")" || echo "Daemon already stopped."
        rm -f "$PID_FILE"
        sleep 1
    fi

    if mount | grep -q -- "$TEST_DIR"; then
        echo "Unmounting $TEST_DIR..."
        fusermount -u "$TEST_DIR" || umount "$TEST_DIR" || echo "Unmount failed, continuing cleanup..."
    fi
    
    if [ -d "$TEST_DIR" ]; then
        echo "Deleting the folder $TEST_DIR"
        rmdir "$TEST_DIR" || echo "Could not remove $TEST_DIR (maybe not empty)."
    fi
}
trap cleanup EXIT ERR

    # --- Preparation ---
echo "--- Preparation of the test environment ---"
# 1. Start the 'wormholed' service
echo "Starting the wormholed service..."
"$WORMHOLE_DAEMON" &
echo "$!" > "$PID_FILE"
echo "Daemon started with PID $(cat "$PID_FILE")"
sleep 2
# 2. Create the mount point
mkdir -p "$TEST_DIR"
# 3. Use the CLI to mount the pod
echo "Creating the pod 'testpod' and mounting FUSE on $TEST_DIR..."
"$WORMHOLE_CLI" new testpod -p 5000 -m "$TEST_DIR"
# 4. Wait for the mount
echo "Waiting for the mount..."
sleep 3
if ! mount | grep -q -- "$TEST_DIR"; then
    echo "ERROR : The FUSE mount point could not be detected!"
    exit 1
fi
echo "FUSE mount detected on $TEST_DIR."
# 5. Create scratch directory
echo "Creating scratch directory at $SCRATCH_MNT..."
mkdir -p "$SCRATCH_MNT"


# --- Execution of the Tests ---
echo "--- Execution of xfstests ---"

# Déplacez-vous dans le répertoire xfstests
cd /opt/xfstests-dev

# --- NOUVELLE CONFIGURATION (basée sur README.fuse) ---
echo "Creating local.config file based on README.fuse..."
cat << EOF > local.config
# --- Configuration FUSE (basée sur README.fuse) ---
export FSTYP="${FSTYP}"
export FUSE_SUBTYP="wormhole" 

# Placeholders (comme dans le README)
export TEST_DEV="non1"
export SCRATCH_DEV="non2"

# Nos vrais chemins
export TEST_DIR="${TEST_DIR}"
export SCRATCH_MNT="${SCRATCH_MNT}"

# Nos overrides pour FUSE pré-monté
export MKFS_PROG="/bin/true"
export MOUNT_PROG="/tests/xfstests_noop_mount.sh"
export UMOUNT_PROG="/tests/xfstests_noop_mount.sh"

# Options du README (au cas où)
export MOUNT_OPTIONS="-osource=${TEST_DIR},allow_other,default_permissions"
export TEST_FS_MOUNT_OPTS="-osource=${TEST_DIR},allow_other,default_permissions"
EOF

echo "local.config created:"
cat local.config
echo "---------------------"

# --- GESTION D'ERREUR (Corrigée) ---
TEST_TO_RUN="generic/001"
LOG_FILE_PATH="results/$TEST_TO_RUN.log"
CHECK_OUTPUT_FILE="/tmp/check_output.log"

echo "Running xfstests test '$TEST_TO_RUN'..."
rm -f "$LOG_FILE_PATH" "$CHECK_OUTPUT_FILE"


# Vérifiez le code de sortie.
if ! ./check -T "$TEST_TO_RUN" > "$CHECK_OUTPUT_FILE" 2>&1; then
    echo "-----------------------------------------------------"
    echo "ERROR: xfstests FAILED with exit code $?"
    echo "-----------------------------------------------------"
    
    echo "Displaying captured output from './check' (from $CHECK_OUTPUT_FILE):"
    echo "--- START OF CAPTURED OUTPUT ---"
    cat "$CHECK_OUTPUT_FILE" || echo "Captured output file not found."
    echo "--- END OF CAPTURED OUTPUT ---"
    
    echo "---"
    echo "Checking for xfstests log file ($LOG_FILE_PATH)..."
    if [ -f "$LOG_FILE_PATH" ]; then
        echo "--- START OF LOG FILE ---"
        cat "$LOG_FILE_PATH"
        echo "--- END OF LOG FILE ---"
    else
        echo "XFSTESTS Log file not found. Dumping 'results' dir:"
        ls -l results/ || echo "'results' directory not found."
    fi
    
    exit $?
fi

echo "--- Test '$TEST_TO_RUN' finished successfully ---"
echo "Log file is at $LOG_FILE_PATH:"
echo "---------------------"
cat "$LOG_FILE_PATH"
echo "---------------------"