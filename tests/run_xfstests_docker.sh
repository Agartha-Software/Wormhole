#!/bin/bash
set -e

# --- Configuration ---
WORMHOLE_DAEMON="/bin/wormholed"
PID_FILE="/test/fuse.pid"

# --- Cleanup function ---
cleanup() {
    echo "--- Cleanup ---"
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
echo "Starting the wormholed service..."
"$WORMHOLE_DAEMON" > /tmp/wormholed.log 2>&1 &
echo "$!" > "$PID_FILE"
echo "Daemon started with PID $(cat "$PID_FILE")"
sleep 2

# --- Execution of the Tests ---
echo "--- Execution of xfstests ---"
cd /opt/xfstests-dev

# Par défaut, xfstests ne connait que fuse.glusterfs et fuse.ceph-fuse.
# On ajoute notre remplacement sed dans la fonction _fs_type de common/rc
sed -i 's|fuse.ceph-fuse/ceph-fuse/|fuse.ceph-fuse/ceph-fuse/;s/fuse.wormhole/fuse/|' common/rc
echo "Patched common/rc to support fuse.wormhole"

touch /tmp/test_dev_file
touch /tmp/scratch_dev_file

# --- Create local.config ---
echo "Creating local.config file..."
cat << EOF > local.config
export FSTYP=fuse
export FUSE_SUBTYP=".wormhole"

# xfstests vérifie si ces noms apparaissent dans 'findmnt'.
# Comme ton code Rust force le nom du FS (FSName) à "wormhole@<dossier>",
# on doit utiliser ces noms exacts ici pour que la vérification réussisse.
export TEST_DEV="wormhole@test"
export SCRATCH_DEV="wormhole@scratch"

export TEST_DIR="/mnt/test"
export SCRATCH_MNT="/mnt/scratch"

export TEST_FS_MOUNT_OPTS="-osubtype=wormhole,pod_name=testpod,port=5000,allow_other,default_permissions"
export MOUNT_OPTIONS="-osubtype=wormhole,pod_name=scratchpod,port=5001,allow_other,default_permissions"

# Empêcher les vérifications de block device
export RESULT_BASE="\$PWD/results"
EOF

# --- Launch the test ---
TEST_TO_RUN="generic/001"
LOG_FILE_PATH="results/$TEST_TO_RUN.log"

# --- Extraction de la description depuis l'en-tête du fichier de test ---
TEST_FILE="tests/$TEST_TO_RUN"

if [ -f "$TEST_FILE" ]; then
    # 1. Cherche "FS QA Test No."
    # 2. Continue tant que les lignes commencent par "#"
    # 3. Retire les "# " du début pour l'affichage
    TEST_DESCRIPTION=$(awk '
        /^# FS QA Test No\./ {start=1; next} 
        !/^#/ {if (start) exit} 
        start {sub(/^# ?/, ""); print}
    ' "$TEST_FILE")
else
    TEST_DESCRIPTION="Test file not found ($TEST_FILE)."
fi

# Si la description est vide (cas où le header est différent), message par défaut
if [ -z "$TEST_DESCRIPTION" ]; then
    TEST_DESCRIPTION="No description found in test header."
fi

echo "-----------------------------------------------------"
echo "Starting test: $TEST_TO_RUN"
echo "-----------------------------------------------------"
echo "Description:"
echo "$TEST_DESCRIPTION"
echo "-----------------------------------------------------"

rm -f "$LOG_FILE_PATH" "results/.config" /tmp/mount_helper.log

# 1. Disable 'set -e' temporarily
set +e

# 2. Execute the command:
./check -fuse -d "$TEST_TO_RUN"

# 3. Capture the TRUE exit code of the first command in the pipe (./check)
EXIT_CODE=${PIPESTATUS[0]}

# 4. Reactivate 'set -e'
set -e

# 5. Verify failure (Exit code non-zero OR specific failure message in output)
if [[ $EXIT_CODE -ne 0 ]]; then
    echo "-----------------------------------------------------"
    echo "ERROR: xfstests FAILED with exit code $EXIT_CODE"
    echo "-----------------------------------------------------"
    
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
    
    # DISPLAY THE DIFF IN CASE OF ERROR (Very important to understand why)
    if [ -f "results/$TEST_TO_RUN.out.bad" ]; then
        echo "--- TEST DIFF (Expected vs Actual) ---"
        # Display the diff but prevent the script from crashing if diff returns a exit code 1
        diff -u "tests/$TEST_TO_RUN.out" "results/$TEST_TO_RUN.out.bad" || true
        echo "-----------------------------------------"
    fi
    
    echo "---"
    echo "Checking mount helper log (/tmp/mount_helper.log)..."
    cat /tmp/mount_helper.log || echo "Mount helper log not found."
    exit 1
fi

# If we arrive here, the test has succeeded
echo "--- Test '$TEST_TO_RUN' finished successfully ---"