#!/bin/sh

set -o errexit

. ./function.sh

cargo build
mkdir -p log

export RUST_LOG=trace
export RUST_BACKTRACE=full

echo "****************************************"
echo "Killing all running $SERVER_NAME server"
sleep 1
kill_all_nodes
echo "****************************************"
echo

echo "****************************************"
echo "Start 3 uninitialized $SERVER_NAME servers..."
nohup ./target/debug/$SERVER_NAME  --id 5001 --addr $DEFAULT_IP:21001 > log/n1.log &
echo "Server 1 started"
nohup ./target/debug/$SERVER_NAME  --id 5002 --addr $DEFAULT_IP:21002 > log/n2.log &
echo "Server 2 started"
nohup ./target/debug/$SERVER_NAME  --id 5003 --addr $DEFAULT_IP:21003 > log/n3.log &
echo "Server 3 started"
echo "Initialize servers 1,2,3 as a 3-nodes cluster"
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Add node 1~3 to the cluster"
rpc 21001/init '[[5001, "127.0.0.1:21001"], [5002, "127.0.0.1:21002"], [5003, "127.0.0.1:21003"]]'
echo "Server 1 is a leader now"
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Get metrics from the leader"
sleep 1
rpc 21001/metrics
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Write data on leader with curl"
sleep 1
rpc 21001/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 50,"payload": "hello"}'
sleep 1
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Write data on leader with tcp_client for 3 times"
sleep 1
echo
sleep 1
./target/debug/tcp_client
sleep 1
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Read data from leader with curl"
sleep 1
rpc 21001/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21001/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21001/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21001/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21001/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Killing all nodes..."
kill_all_nodes
echo "Done"
echo "****************************************"
echo
sleep 2

echo "Test complete"