#!/bin/sh

# === Script de déploiement Wormhole sur Kubernetes ===
#
# Ce script automatise les étapes suivantes :
# 1. Création (optionnelle) d'un cluster 'kind'
# 2. Demande des identifiants GHCR pour l'image privée
# 3. Création du namespace et du secret d'image
# 4. Déploiement du StatefulSet depuis 'wormhole.yaml'
# 5. Configuration des 3 pods pour qu'ils se connectent en réseau

# Quitte immédiatement si une commande échoue
set -e

# --- Étape 0: Création du cluster 'kind' (Optionnel) ---
echo "=== Étape 0: Cluster 'kind' ==="
read -p "Voulez-vous créer un nouveau cluster 'kind' nommé 'wormhole' ? (o/n) " CREATE_KIND
if [[ "$CREATE_KIND" == "o" ]]; then
  
  echo "Vérification de l'existence d'un cluster 'kind' nommé 'wormhole'..."
  # On vérifie si 'kind get clusters' retourne une ligne qui est EXACTEMENT 'wormhole'
  if kind get clusters | grep -q "^wormhole$"; then
    echo "-> Un cluster existant 'wormhole' a été trouvé. Suppression..."
    kind delete cluster --name wormhole
    echo "-> Cluster 'wormhole' supprimé."
  else
    echo "-> Aucun cluster existant trouvé."
  fi

  echo "Création du cluster 'kind-wormhole'..."
  kind create cluster --name wormhole
  kubectl cluster-info --context kind-wormhole
else
  echo "Skipping cluster creation. Utilisation du contexte 'kubectl' actuel."
fi

# --- Étape 1: Création du Secret Docker ---
echo ""
echo "=== Étape 1: Secret Docker (ghcr.io) ==="
# Détermine le répertoire absolu où se trouve le script
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
ENV_FILE="$SCRIPT_DIR/.env"

echo "Recherche du fichier .env dans: $ENV_FILE"
if [ -f "$ENV_FILE" ]; then
  # 'set -a' exporte toutes les variables définies dans le .env
  # 'set +a' arrête ce comportement
  set -a
  source "$ENV_FILE"
  set +a
  echo "Fichier .env chargé."
else
  echo "ERREUR: Fichier .env introuvable."
  echo "Veuillez créer un fichier .env contenant votre GITHUB_TOKEN."
  echo "Exemple: GITHUB_TOKEN=ghp_votrejetonpersonnel"
  exit 1
fi

# Vérifie que le GITHUB_TOKEN a bien été chargé
if [ -z "$GITHUB_TOKEN" ]; then
  echo "ERREUR: GITHUB_TOKEN est vide ou n'est pas défini dans votre fichier .env."
  exit 1
fi

# # Vérifie que le GITHUB_USERNAME a bien été chargé
# if [ -z "$GITHUB_USERNAME" ]; then
#   echo "ERREUR: GITHUB_USERNAME est vide ou n'est pas défini dans votre fichier .env."
#   exit 1
# fi
# 
# # Vérifie que le EMAIL a bien été chargé
# if [ -z "$EMAIL" ]; then
#   echo "ERREUR: EMAIL est vide ou n'est pas défini dans votre fichier .env."
#   exit 1
# fi

echo "Veuillez saisir votre nom d'utilisateur GitHub pour 'ghcr.io'."
read -p "Nom d'utilisateur GitHub: " GITHUB_USERNAME

echo "Veuillez saisir votre email pour 'ghcr.io'."
read -p "Email: " EMAIL

echo "Création du namespace 'wormhole'..."
kubectl create namespace wormhole --dry-run=client -o yaml | kubectl apply -f -

echo "Suppression de l'ancien secret 'ghcr-creds' (si existant)..."
kubectl -n wormhole delete secret docker-registry ghcr-creds || true

echo "Création du nouveau secret 'ghcr-creds'..."
kubectl -n wormhole create secret docker-registry ghcr-creds \
  --docker-server=ghcr.io \
  --docker-username="$GITHUB_USERNAME" \
  --docker-password="$GITHUB_TOKEN" \
  --docker-email="$EMAIL"

echo "Secret 'ghcr-creds' créé avec succès."

# --- Étape 2: Déploiement de Wormhole ---
echo ""
echo "=== Étape 2: Déploiement de 'wormhole.yaml' ==="
kubectl apply -f wormhole.yaml

echo "En attente que les 3 pods soient prêts..."
kubectl wait --for=condition=ready pod \
  -l app=wormhole \
  -n wormhole \
  --timeout=300s
echo "Les 3 pods sont 'Running'."

# --- Étape 3: Configuration du réseau ---
echo ""
echo "=== Étape 3: Configuration du réseau Wormhole ==="

echo "Configuration de 'wormhole-0' (Nœud 1)..."
kubectl -n wormhole exec wormhole-0 -- bash -c \
  "mkdir -p /wormhole/whfolder && wormhole new pod1 -p 40001 -m /wormhole/whfolder"

echo "Récupération de l'IP de 'wormhole-0' pour les autres nœuds..."
PEER_IP=$(kubectl -n wormhole exec wormhole-0 -- getent hosts wormhole-0.wormhole | awk '{ print $1 }')

if [[ -z "$PEER_IP" ]]; then
  echo "ERREUR: Impossible de récupérer l'IP de 'wormhole-0'. Arrêt."
  exit 1
fi
echo "IP de 'wormhole-0' trouvée: $PEER_IP"

# Configuration de wormhole-1
echo "Configuration de 'wormhole-1' (Nœud 2) pour rejoindre $PEER_IP:40001..."
kubectl -n wormhole exec wormhole-1 -- bash -c \
  "mkdir -p /wormhole/whfolder && wormhole new pod2 -p 40002 -m /wormhole/whfolder -u ${PEER_IP}:40001"

echo "Récupération de l'IP de 'wormhole-1'..."
PEER1_IP=$(kubectl -n wormhole exec wormhole-1 -- getent hosts wormhole-1.wormhole | awk '{ print $1 }')

if [[ -z "$PEER1_IP" ]]; then
  echo "ERREUR: Impossible de récupérer l'IP de 'wormhole-1'. Arrêt."
  exit 1
fi
echo "IP de 'wormhole-1' trouvée: $PEER1_IP"

# Configuration de wormhole-2
echo "Configuration de 'wormhole-2' (Nœud 3) pour rejoindre $PEER1_IP:40002..."
kubectl -n wormhole exec wormhole-2 -- bash -c \
  "mkdir -p /wormhole/whfolder && wormhole new pod3 -p 40003 -m /wormhole/whfolder -u ${PEER1_IP}:40002"

echo ""
echo "✅ Succès ! Les 3 pods wormhole sont déployés et devraient être connectés."
echo "Vous pouvez vérifier l'état en vous connectant à un pod :"
echo "kubectl -n wormhole exec -it wormhole-0 -- /bin/bash"