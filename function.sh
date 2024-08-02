#!/bin/sh

SERVER_NAME="aizumi"
DEFAULT_IP="127.0.0.1"

kill_all_nodes() {
    if [ "$(uname)" = "Darwin" ]; then
        SERVICE=$SERVER_NAME
        if pgrep -xq -- "${SERVICE}"; then
            pkill -f "${SERVICE}"
        fi
    else
        set +e # killall will error if finds no process to kill
        killall $SERVER_NAME
        set -e
    fi
}

rpc() {
    local uri=$1
    local body="$2"

    echo '---'" rpc(:$uri, $body)"

    {
        if [ ".$body" = "." ]; then
            # time curl --silent "$DEFAULT_IP:$uri"
            curl --silent "$DEFAULT_IP:$uri"
        else
            # time curl --silent "$DEFAULT_IP:$uri" -H "Content-Type: application/json" -d "$body"
            curl --silent "$DEFAULT_IP:$uri" -H "Content-Type: application/json" -d "$body"
        fi
    } | {
        if type jq > /dev/null 2>&1; then
            # jq
            cat
        else
            cat
        fi
    }

    echo
    echo
}
