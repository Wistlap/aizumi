#!/bin/sh

set -o errexit

SERVER_NAME="aizumi"
DEFAULT_IP="127.0.0.1"

cargo build

kill() {
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

export RUST_LOG=trace
export RUST_BACKTRACE=full

echo "****************************************"
echo "Killing all running $SERVER_NAME server"
kill
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Start 5 uninitialized $SERVER_NAME servers..."
echo "---"
nohup ./target/debug/$SERVER_NAME  --id 5001 --addr $DEFAULT_IP:21001 > log/n1.log &
sleep 1
echo "Server 1 started"
nohup ./target/debug/$SERVER_NAME  --id 5002 --addr $DEFAULT_IP:21002 > log/n2.log &
sleep 1
echo "Server 2 started"
nohup ./target/debug/$SERVER_NAME  --id 5003 --addr $DEFAULT_IP:21003 > log/n3.log &
sleep 1
echo "Server 3 started"
nohup ./target/debug/$SERVER_NAME  --id 5004 --addr $DEFAULT_IP:21004 > log/n4.log &
sleep 1
echo "Server 4 started"
nohup ./target/debug/$SERVER_NAME  --id 5005 --addr $DEFAULT_IP:21005 > log/n5.log &
sleep 1
echo "Server 5 started"
echo "---"
echo "Initialize servers 1,2,3 as a 3-nodes cluster"
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Add node 1~3 to the cluster"
echo "---"
rpc 21001/init '[[5001, "127.0.0.1:21001"], [5002, "127.0.0.1:21002"], [5003, "127.0.0.1:21003"]]'
echo "Server 1 is a leader now"
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Get metrics from the leader"
echo "---"
sleep 1
rpc 21001/metrics
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Adding node 4 and node 5 as learners, to receive log from leader node 1"
echo "---"
echo
sleep 1
rpc 21001/add-learner       '[5004, "127.0.0.1:21004"]'
echo "Node 4 added as learner"
sleep 1
rpc 21001/add-learner       '[5005, "127.0.0.1:21005"]'
echo "Node 5 added as learner"
sleep 1
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Get metrics from the leader, after adding 2 learners"
echo "---"
sleep 1
rpc 21001/metrics
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Changing membership from [5001, 5002, 5003] to 5 nodes cluster: [5001, 5002, 5003, 5004, 5005]"
echo "---"
rpc 21001/change-membership '[5001, 5002, 5003, 5004, 5005]'
sleep 1
echo 'Membership changed to [5001, 5002, 5003, 5004, 5005]'
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Get metrics from the leader again"
echo "---"
sleep 1
rpc 21001/metrics
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Write data on leader"
sleep 1
echo "---"
rpc 21001/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 1,"payload": "hello"}'
sleep 1
rpc 21001/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 2,"payload": "hello"}'
sleep 1
rpc 21001/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 3,"payload": "hello"}'
sleep 1
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Read data from leader"
echo "---"
sleep 1
rpc 21001/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Changing membership from [5001, 5002, 5003, 5004, 5005] to [5003]"
echo "---"
sleep 1
rpc 21001/change-membership '[5003]'
sleep 1
echo "---"
echo 'Membership changed to [5003]'
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Get metrics from the node-3"
echo "---"
sleep 1
rpc 21003/metrics
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Write Data on node-3"
echo "---"
sleep 1
rpc 21003/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 4,"payload": "hello"}'
sleep 1
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Read data from node-3"
echo "---"
sleep 1
rpc 21003/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21003/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Killing all nodes..."
kill
echo "Done"
echo "****************************************"
echo
sleep 1

echo "Test complete"
