#!/usr/bin/env bash

# === Wormhole Kubernetes Deployment Script ===
#
# This script automates the following steps:
# 1. Optional 'kind' cluster creation (cleans up old one if found)
# 2. Asks for GHCR credentials for the private image
# 3. Creates the namespace and image pull secret
# 4. Deploys the StatefulSet from 'wormhole.yaml'
# 5. Configures the 3 pods to connect in a chain (0 <- 1 <- 2)

# Exit immediately if a command fails
set -e

# --- Color Definitions ---
# Use direct string assignment, which is more robust
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

# Check if stdout is not a terminal (e.g., in a file) and disable colors
if [ ! -t 1 ]; then
    RED=""
    GREEN=""
    YELLOW=""
    CYAN=""
    BOLD=""
    RESET=""
fi

# Helper function for safe printing
# Usage: print_color "$GREEN" "This is green text"
print_color() {
    local color="$1"
    local message="$2"
    # CORRECTION: The color codes are part of the format string,
    # and the message is passed as a safe '%s' argument.
    # This forces printf to interpret the escape codes.
    printf "$color%s$RESET\n" "$message"
}

# Helper function for printing standard text
# Safely prints strings, even if they start with '-'
print() {
    printf "%s\n" "$1"
}


# --- Step 0: Kind Cluster Setup ---
print_color "$BOLD$CYAN" "=== Step 0: Kind Cluster Setup ==="

print_color "$YELLOW" "Checking for existing 'kind' cluster named 'wormhole'..."
# Check if 'kind get clusters' returns a line that is EXACTLY 'wormhole'
if kind get clusters | grep -q "^wormhole$"; then
  print "-> Found existing 'wormhole' cluster. Deleting..."
  kind delete cluster --name wormhole
  print_color "$GREEN" "-> Cluster 'wormhole' deleted."
else
  print "-> No existing cluster found."
fi

print_color "$YELLOW" "Creating new 'kind-wormhole' cluster..."
kind create cluster --name wormhole
kubectl cluster-info --context kind-wormhole

# --- Step 1: Docker Secret Creation ---
printf "\n" # Add a newline
print_color "$BOLD$CYAN" "=== Step 1: Docker Secret (ghcr.io) ==="
# Determine the absolute directory where the script is located
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
ENV_FILE="$SCRIPT_DIR/.env"

print_color "$YELLOW" "Looking for .env file at: $ENV_FILE"
if [ -f "$ENV_FILE" ]; then
  set -a
  source "$ENV_FILE"
  set +a
  print_color "$GREEN" "Loaded .env file."
else
  print_color "$RED" "ERROR: .env file not found."
  print "Please create a .env file containing your GITHUB_TOKEN."
  print "Example: GITHUB_TOKEN=ghp_yourpersonaltoken"
  exit 1
fi

# Check that GITHUB_TOKEN was loaded
if [ -z "$GITHUB_TOKEN" ]; then
  print_color "$RED" "ERROR: GITHUB_TOKEN is empty or not set in your .env file."
  exit 1
fi

# Get GITHUB_USERNAME interactively
print_color "$YELLOW" "Please enter your GitHub username for 'ghcr.io'."
read -p "GitHub Username: " GITHUB_USERNAME

# Clean up variables from potential \r (Windows line endings)
GITHUB_TOKEN=$(echo "$GITHUB_TOKEN" | tr -d '\r')
GITHUB_USERNAME=$(echo "$GITHUB_USERNAME" | tr -d '\r')

print_color "$YELLOW" "Creating namespace 'wormhole'..."
kubectl create namespace wormhole --dry-run=client -o yaml | kubectl apply -f -

print_color "$YELLOW" "Deleting old 'ghcr-creds' secret (if it exists)..."
kubectl -n wormhole delete secret docker-registry ghcr-creds || true

print_color "$YELLOW" "Creating new 'ghcr-creds' secret..."
# ** CRITICAL: Removed --docker-email as it's not needed for ghcr.io and breaks auth **
kubectl -n wormhole create secret docker-registry ghcr-creds \
  --docker-server=ghcr.io \
  --docker-username="$GITHUB_USERNAME" \
  --docker-password="$GITHUB_TOKEN"

print_color "$GREEN" "Secret 'ghcr-creds' created successfully."

# --- Step 2: Deploy Wormhole ---
printf "\n"
print_color "$BOLD$CYAN" "=== Step 2: Deploying 'wormhole.yaml' ==="
kubectl apply -f "$SCRIPT_DIR/wormhole.yaml"

print_color "$YELLOW" "Waiting for all 3 pods to be ready..."
kubectl wait --for=condition=ready pod \
  -l app=wormhole \
  -n wormhole \
  --timeout=300s
print_color "$GREEN" "All 3 pods are 'Running'."

# --- Step 3: Network Configuration ---
printf "\n"
print_color "$BOLD$CYAN" "=== Step 3: Configuring Wormhole Network (Chain) ==="

print_color "$YELLOW" "Configuring 'wormhole-0' (Node 1) on port 40001..."
kubectl -n wormhole exec wormhole-0 -- bash -c \
  "mkdir -p /wormhole/whfolder && wormhole new pod1 -p 40001 -m /wormhole/whfolder"

print_color "$YELLOW" "Fetching IP for 'wormhole-0' for other nodes..."
PEER0_IP=$(kubectl -n wormhole exec wormhole-0 -- getent hosts wormhole-0.wormhole | awk '{ print $1 }')

if [[ -z "$PEER0_IP" ]]; then
  print_color "$RED" "ERROR: Could not get IP for 'wormhole-0'. Aborting."
  exit 1
fi
printf "%sIP for 'wormhole-0' found: %s%s%s\n" "$RESET" "$BOLD" "$PEER0_IP" "$RESET"


# Configure wormhole-1
print_color "$YELLOW" "Configuring 'wormhole-1' (Node 2) on port 40002..."
print "  -> Connecting to wormhole-0 ($PEER0_IP:40001)"
kubectl -n wormhole exec wormhole-1 -- bash -c \
  "mkdir -p /wormhole/whfolder && wormhole new pod2 -p 40002 -m /wormhole/whfolder -u ${PEER0_IP}:40001"

print_color "$YELLOW" "Fetching IP for 'wormhole-1'..."
PEER1_IP=$(kubectl -n wormhole exec wormhole-1 -- getent hosts wormhole-1.wormhole | awk '{ print $1 }')

if [[ -z "$PEER1_IP" ]]; then
  print_color "$RED" "ERROR: Could not get IP for 'wormhole-1'. Aborting."
  exit 1
fi
printf "%sIP for 'wormhole-1' found: %s%s%s\n" "$RESET" "$BOLD" "$PEER1_IP" "$RESET"

# Configure wormhole-2
print_color "$YELLOW" "Configuring 'wormhole-2' (Node 3) on port 40003..."
print "  -> Connecting to wormhole-1 ($PEER1_IP:40002)"
kubectl -n wormhole exec wormhole-2 -- bash -c \
  "mkdir -p /wormhole/whfolder && wormhole new pod3 -p 40003 -m /wormhole/whfolder -u ${PEER1_IP}:40002"

printf "\n"
print_color "$GREEN" "✅ Success! All 3 wormhole pods are deployed and should be connected."
print "You can check the status by exec-ing into a pod:"
print_color "$BOLD" "kubectl -n wormhole exec -it wormhole-0 -- /bin/bash"