#!/bin/bash

trap 'kill $(jobs -p)' EXIT

is_port_open() {
    nc -z localhost $1
}

frontend_dev() {
    cd crates/web-app/frontend
    if command -v nvm &> /dev/null; then
        nvm use
    fi
    if ! command -v node &> /dev/null; then
        echo "node is not installed, please install it first"
        exit 1
    fi
    if ! command -v pnpm &> /dev/null; then
        echo "pnpm is not installed, please install it first"
        exit 1
    fi
    pnpm dev
}

api_dev() {
    cargo watch -x run
}

case $1 in
    frontend|fe)
        if is_port_open 5173; then
            echo "Port 5173 is already in use, maybe frontend is already running?"
            exit 1
        fi
        frontend_dev
        ;;
    api|ap|app)
        if is_port_open 3000; then
            echo "Port 3000 is already in use, maybe api is already running?"
            exit 1
        fi
        api_dev
        ;;
    proxy|caddy|p)
        if is_port_open 8080; then
            echo "Port 8080 is already in use, maybe proxy is already running?"
            exit 1
        fi
        caddy run --config Caddyfile
        ;;
    *)
        echo "Usage: $0 frontend|api|proxy"
        ;;
esac
