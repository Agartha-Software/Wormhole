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
"$WORMHOLE_DAEMON" &
echo "$!" > "$PID_FILE"
echo "Daemon started with PID $(cat "$PID_FILE")"
sleep 2

# --- Execution of the Tests ---
echo "--- Execution of xfstests ---"
cd /opt/xfstests-dev

# --- Créer local.config (Corrigé) ---
echo "Creating local.config file..."
cat << EOF > local.config
# 1. FSTYP est 'fuse' pour que xfstests le reconnaisse
export FSTYP="fuse"
export FUSE_SUBTYP=".wormhole"
# (On supprime FUSE_SUBTYP, il est inutile et crée des conflits)

# 2. Placeholders (acceptés car FSTYP=fuse)
export TEST_DEV="non1"
export SCRATCH_DEV="non2"

# 3. Dossiers standards
export TEST_DIR="/mnt/test"
export SCRATCH_MNT="/mnt/scratch"

# 4. Options de montage avec 'subtype=wormhole'
#    pour que /sbin/mount.fuse appelle /sbin/mount.fuse.wormhole
export TEST_FS_MOUNT_OPTS="-osubtype=wormhole,pod_name=testpod,port=5000,allow_other,default_permissions"
export MOUNT_OPTIONS="-osubtype=wormhole,pod_name=scratchpod,port=5001,allow_other,default_permissions"
EOF

echo "local.config created:"
cat local.config
echo "---------------------"

# --- Lancement du test (Capture d'erreur Corrigée) ---
TEST_TO_RUN="generic/001"
LOG_FILE_PATH="results/$TEST_TO_RUN.log"
CHECK_OUTPUT_FILE="/tmp/check_output.log"

echo "Running xfstests test '$TEST_TO_RUN'..."
rm -f "$LOG_FILE_PATH" "$CHECK_OUTPUT_FILE" "results/.config" /tmp/mount_helper.log

# -- CORRECTION DU BUG "exit code 0" --
# 1. Désactiver 'set -e' temporairement
set +e
# 2. Exécuter la commande et rediriger la sortie
./check "$TEST_TO_RUN" > "$CHECK_OUTPUT_FILE" 2>&1
# 3. Capturer le VRAI code de sortie
EXIT_CODE=$?
# 4. Réactiver 'set -e'
set -e

# 5. Vérifier le vrai code de sortie
if [ "$EXIT_CODE" -ne 0 ]; then
    echo "-----------------------------------------------------"
    echo "ERROR: xfstests FAILED with exit code $EXIT_CODE"
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
    
    echo "---"
    echo "Checking mount helper log (/tmp/mount_helper.log)..."
    cat /tmp/mount_helper.log || echo "Mount helper log not found."
    
    exit $EXIT_CODE
fi

# Si on arrive ici, le test a réussi
echo "--- Test '$TEST_TO_RUN' finished successfully ---"
echo "Log file is at $LOG_FILE_PATH:"
echo "---------------------"
cat "$LOG_FILE_PATH"
echo "---------------------"