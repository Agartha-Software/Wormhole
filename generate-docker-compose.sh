#!/bin/bash

# Demander à l'utilisateur le nombre de services à lancer
read -p "Combien de services voulez-vous lancer ? (min 1, max 10) " SERVICE_COUNT

# Vérifier que le nombre est valide
if [[ $SERVICE_COUNT -lt 1 || $SERVICE_COUNT -gt 10 ]]; then
    echo "Nombre de services invalide. Veuillez choisir un nombre entre 1 et 10."
    exit 1
fi

# Créer le fichier docker-compose.yml
cat <<EOF > docker-compose.yml
services:
  wormhole-base:
    &base-service
    build: .
    privileged: true
    cap_add:
      - SYS_ADMIN
    devices:
      - '/dev/fuse'
    volumes:
      - ./shared_mnt:/usr/src/wormhole/virtual:rwx
    stdin_open: true
    tty: true
    networks:
      - wormhole-net
    environment:
      RUST_BACKTRACE: "1"
EOF

# Ajouter les services dynamiquement
for ((i=1; i<=SERVICE_COUNT; i++)); do
    # Créer le dossier partagé
    mkdir -p shared_mnt$i

    # Ajout des services
    cat <<EOF >> docker-compose.yml
  wormhole$i:
    <<: *base-service
    container_name: w$i
    networks:
      - wormhole-net
    command: "./wormhole-service 0.0.0.0:8081 >> /var/log/wormhole.log 2>&1"
    volumes:
      - ./shared_mnt$i:/usr/src/wormhole/virtual:rwx
EOF

    # Ajouter depends_on si nécessaire
    if [[ $i -gt 1 ]]; then
        cat <<EOF >> docker-compose.yml
    depends_on:
      - wormhole$((i-1))
EOF
    fi

    # Ajouter la section deploy
    cat <<EOF >> docker-compose.yml
    deploy:
      restart_policy:
        condition: on-failure
        delay: 5s
        max_attempts: 3
        window: 120s
EOF
done

# Ajouter la section réseaux
cat <<EOF >> docker-compose.yml
networks:
  wormhole-net:
    driver: bridge
EOF

# Afficher un message de confirmation
echo "Fichier docker-compose.yml généré avec $SERVICE_COUNT services."

# Vérifier si l'image Docker a déjà été construite
# if docker images | grep -q "wormhole-base"; then
#     echo "L'image Docker existe déjà. Lancement de docker-compose up --no-deps..."
#     docker-compose up --no-deps
# else
#     echo "L'image Docker n'a jamais été construite. Lancement de docker-compose up --build --no-deps..."
# fi
docker-compose up --build --no-deps