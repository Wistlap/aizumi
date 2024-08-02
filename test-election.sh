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
sleep 1

echo "****************************************"
echo "Start 5 uninitialized $SERVER_NAME servers..."
sleep 1
nohup ./target/debug/$SERVER_NAME  --id 5001 --addr $DEFAULT_IP:21001 > log/n1.log &
leader_pid=$!
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
echo "Initialize servers 1,2,3,4,5 as a 5-nodes cluster"
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Add node 1~5 to the cluster"
sleep 1
rpc 21001/init '[[5001, "127.0.0.1:21001"], [5002, "127.0.0.1:21002"], [5003, "127.0.0.1:21003"], [5004, "127.0.0.1:21004"], [5005, "127.0.0.1:21005"]]'
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
echo "Write data on leader"
sleep 1
rpc 21001/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 1,"payload": "hello"}'
sleep 1
rpc 21001/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 2,"payload": "hello"}'
sleep 1
rpc 21001/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 3,"payload": "hello"}'
sleep 1
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Read data from leader"
sleep 1
rpc 21001/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "kill leader process"
sleep 1
kill $leader_pid
sleep 5
echo "Leader server killed"
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Get metrics from the living nodes"
sleep 1
rpc 21002/metrics
sleep 1
rpc 21003/metrics
sleep 1
rpc 21004/metrics
sleep 1
rpc 21005/metrics
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo " Try read data from living nodes (current leader can read data)"
sleep 1
rpc 21002/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21003/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21004/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21005/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
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
