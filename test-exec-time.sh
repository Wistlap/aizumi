#!/bin/bash

set -o errexit

. ./function.sh

cargo build --release
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
echo "Start 3 uninitialized $SERVER_NAME servers..."
sleep 1
nohup ./target/release/$SERVER_NAME  --id 5001 --addr $DEFAULT_IP:21001 > log/n1.log &
sleep 1
echo "Server 1 started"
nohup ./target/release/$SERVER_NAME  --id 5002 --addr $DEFAULT_IP:21002 > log/n2.log &
sleep 1
echo "Server 2 started"
nohup ./target/release/$SERVER_NAME  --id 5003 --addr $DEFAULT_IP:21003 > log/n3.log &
sleep 1
echo "Server 3 started"
echo "Initialize servers 1,2,3 as a 3-nodes cluster"
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Add node 1 to the cluster"
sleep 1
rpc 21001/init '[[5001, "127.0.0.1:21001"]]'
echo "Server 1 is a leader now"
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "Get metrics from the leader"
sleep 1
rpc 21001/metrics
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "http request 1,000 times"
echo "send request"
time for i in $(seq 1 1000); do
    rpc 21001/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 1,"payload": "hello"}' > /dev/null
done
echo

echo "recv request"
time for i in $(seq 1 1000); do
    rpc 21001/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}' > /dev/null
done
echo "****************************************"
sleep 2
echo
echo

echo "****************************************"
echo "tcp request 1,000 times"
echo "send request"
for i in $(seq 1); do
    # echo $i
    ./target/release/tcp_client -m "MSG_SEND_REQ" -s 1 -d 100 -i $i -b "127.0.0.1:21101" -l 1000
done
sleep 1
echo

echo "recv request"
for i in $(seq 1); do
    # echo $i
    ./target/release/tcp_client -m "MSG_RECV_REQ" -s 100 -d 1 -i $i -b "127.0.0.1:21101" -l 1000
done
sleep 2
echo
echo

echo "****************************************"
echo "Get metrics from the leader again"
sleep 1
rpc 21001/metrics
echo "****************************************"
echo
sleep 1


echo "****************************************"
echo "Change membership from [5001] to 5 nodes cluster: [5001, 5002, 5003]"
rpc 21001/add-learner       '[5002, "127.0.0.1:21002"]'
sleep 1
rpc 21001/add-learner       '[5003, "127.0.0.1:21003"]'
sleep 1
rpc 21001/change-membership '[5001, 5002, 5003]'
echo "Done"
echo "****************************************"
echo
sleep 2

echo "****************************************"
echo "Get metrics from the leader again"
sleep 1
rpc 21001/metrics
echo "****************************************"
echo
sleep 1

echo "****************************************"
echo "http request 100 times"
echo "send request"
time for i in $(seq 1 100); do
    # echo $i
    rpc 21001/ '{"msg_type": "MSG_SEND_REQ","saddr": 1,"daddr": 100,"id": 1,"payload": "hello"}' > /dev/null
done
echo

echo "recv request"
time for i in $(seq 1 100); do
    # echo $i
    rpc 21001/ '{"msg_type": "MSG_RECV_REQ","saddr": 100,"daddr": 1, "id": 0, "payload": ""}' > /dev/null
done
echo "****************************************"
sleep 2
echo
echo

echo "****************************************"
echo "tcp request 100 times"
echo "send request"
for i in $(seq 1); do
    # echo $i
    ./target/release/tcp_client -m "MSG_SEND_REQ" -s 1 -d 100 -i $i -b "127.0.0.1:21101" -l 100
done
sleep 1
echo

echo "recv request"
for i in $(seq 1); do
    # echo $i
    ./target/release/tcp_client -m "MSG_RECV_REQ" -s 100 -d 1 -i $i -b "127.0.0.1:21101" -l 100
done
sleep 2
echo
echo


echo "****************************************"
echo "Killing all nodes..."
kill_all_nodes
echo "Done"
echo "****************************************"
echo
sleep 2

echo "Test complete"
