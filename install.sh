#!/bin/bash

api_image="heltec/snapemu-api:main"
manager_image="heltec/snapemu-manager:main"
web_image="heltec/snapemu-web:main"
db_image="postgres:16.3-alpine3.20"
redis_image="redis:7.2.4-alpine3.19"

install_dir=$(pwd)/snapemu
db_password=''
web_domain=''
jwt_key=$(head /dev/urandom | tr -dc A-Za-z0-9 | head -c 16)

RED='\e[31m'
RESET='\e[0m'
GREEN='\e[32m'
error_log() {
    local input="$1"
    echo -e "${RED}${input}${RESET}"
    exit 1
}
info_log() {
    local input="$1"
    echo -e "${GREEN}${input}${RESET}"
}

if [ -f /etc/os-release ]; then
    . /etc/os-release
    OS=$ID
else
    echo "Only Ubuntu and Debian are supported"
    exit 1
fi

install_docker_ubuntu_debian() {
  sudo apt-get update
  sudo apt-get install ca-certificates curl
  sudo install -m 0755 -d /etc/apt/keyrings
  sudo curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc
  sudo chmod a+r /etc/apt/keyrings/docker.asc
  echo \
    "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian \
    $(. /etc/os-release && echo "$VERSION_CODENAME") stable" | \
    sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
  sudo apt-get update
  sudo apt-get -y install docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
}

check_docker_installed() {
    if command -v docker &> /dev/null; then
        local version=$(docker --version)
        info_log "docker is installed：${version}"
    else
      ask_to_install_docker
      check_docker_installed
    fi
}

check_docker_running() {
    if sudo systemctl is-active --quiet docker; then
        info_log "Docker is running"
    else
        echo "Docker is not running, trying to start..."
        sudo systemctl start docker
        if sudo systemctl is-active --quiet docker; then
            info_log "Docker has started successfully"
        else
            error_log "Docker startup failed, please check the problem"
            exit 1
        fi
    fi
}

ask_to_install_docker() {
    local answer
    # 提示用户输入
    echo -n -e "Do you want to install docker? (Y/n)"
    read answer
    answer="${answer:-Y}"
    case "$answer" in
        yes|y|Y)
            ;;
        *)
            error_log "exit setup"
            exit 1
            ;;
    esac
    case "$OS" in
        ubuntu|debian)
            install_docker_ubuntu_debian
            ;;
        *)
            error_log "The operating system does not support it"
            exit 1
            ;;
    esac
}

check_install_dir() {
  if [ -d "$install_dir" ]; then
    error_log "The software has been installed in $install_dir"
  else
    info_log "The software will be installed in ${install_dir}"
    mkdir -p $install_dir
  fi
}

generate_docker_compose() {
    in_china="n"
    read -p "In China? (y/N): " in_china
    if [[ "in_china" == "y" || "$in_china" == "Y" || "$in_china" == "Yes" || "$in_china" == "y" ]]; then
        api_image="registry.cn-hongkong.aliyuncs.com/snap_emu/api:main"
        manager_image="registry.cn-hongkong.aliyuncs.com/snap_emu/manager:main"
        web_image="registry.cn-hongkong.aliyuncs.com/snap_emu/web:main"
        db_image="registry.cn-hongkong.aliyuncs.com/snap_emu/db:main"
        redis_image="registry.cn-hongkong.aliyuncs.com/snap_emu/redis:main"
    fi
  cat <<EOF > ${install_dir}/compose.yaml
services:
  db:
    image: "${db_image}"
    restart: always
    environment:
      POSTGRES_PASSWORD: ${db_password}
      POSTGRES_DB: snapemu
    volumes:
      - "db-data:/var/lib/postgresql/data"
  redis:
    image: "${redis_image}"
    restart: always
  api:
    image: "${api_image}"
    pull_policy: always
    volumes:
      - ./config:/etc/snapemu:ro
      - "assets:/app/assets"
    restart: always
    command: ["./snap_api", "run"]
  manager:
    image: "${manager_image}"
    pull_policy: always
    ports:
      - 1700:1700/udp
    volumes:
      - ./config:/etc/snapemu:ro
    restart: always
    command: ["./devices_manager", "run"]
  web:
    image: "${web_image}"
    pull_policy: always
    restart: always
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
    ports:
      - "80:80"
      - "443:443"
volumes:
  db-data:
  assets:
EOF
}

generate_config() {
  local config_dir=${install_dir}/config
  if [ -d "$config_dir" ]; then
    error_log "The software has been installed in ${config_dir}"
  else
    mkdir -p $config_dir
  fi
  local api_config=${install_dir}/config/config.yaml
  local caddy_config=${install_dir}/Caddyfile
  read -p "Enter the website domain name: " web_domain
  read -p "Please enter the database password: " db_password
  cat <<EOF > ${api_config}
log: INFO
device:
  topic:
    data: LoRa-Push-Data
    event: LoRa-Event
    down: LoRa-Down-Data
  lorawan:
    host: "0.0.0.0"
    port: 1700
api:
  model:
    path: /etc/snapemu/model.yaml
  port: 8000
  host: "0.0.0.0"
  web: "${web_domain}"
redis:
  host: redis
  db: 0
db:
  host: db
  password: ${db_password}
  username: postgres
  db: snapemu
jwt_key: ${jwt_key}
EOF

  cat <<EOF > ${caddy_config}
:80 {
	route /api/* {
          reverse_proxy http://api:8000
        }
	route {
          root * /var/www/html
          try_files {path} /index.html
          file_server
          encode    zstd   gzip
        }
}
EOF
}


start_run() {
  cd $install_dir
  sudo docker compose up -d
}

check_docker_installed
check_docker_running
check_install_dir
generate_config
generate_docker_compose
start_run
