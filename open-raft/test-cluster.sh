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
echo "Initialize servers 1,2,3 as a 3-nodes cluster"
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Add node 1~3 to the cluster"
sleep 1
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
echo "Adding node 4 and node 5 as learners, to receive log from leader node 1"
sleep 1
rpc 21001/add-learner       '[5004, "127.0.0.1:21004"]'
echo "Node 4 added as learner"
sleep 1
rpc 21001/add-learner       '[5005, "127.0.0.1:21005"]'
echo "Node 5 added as learner"
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Get metrics from the leader, after adding 2 learners"
sleep 1
rpc 21001/metrics
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Changing membership from [5001, 5002, 5003] to 5 nodes cluster: [5001, 5002, 5003, 5004, 5005]"
sleep 1
rpc 21001/change-membership '[5001, 5002, 5003, 5004, 5005]'
echo 'Membership changed to [5001, 5002, 5003, 5004, 5005]'
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Get metrics from the leader again"
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
echo "Changing membership from [5001, 5002, 5003, 5004, 5005] to [5003]"
sleep 1
rpc 21001/change-membership '[5003]'
echo 'Membership changed to [5003]'
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Get metrics from the node-3"
sleep 1
rpc 21003/metrics
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Write Data on node-3 (current leader)"
sleep 1
rpc 21003/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 4,"payload": "hello"}'
sleep 1
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Read data from node-3 (current leader)"
sleep 1
rpc 21003/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21003/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21003/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
sleep 1
rpc 21003/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}'
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
