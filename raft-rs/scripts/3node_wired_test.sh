#!/usr/bin/env bash

##########
# This script is used when testing Raft communication between
# different computers.
# Manually start the leader on one computer, A, and use this
# script on the other computer, B.
# The command line arguments specify the number of senders -s
# and the number of receivers -r.
##########

function is_receiver_working() {
    test $(ps | grep m-receiver | wc -l) != 0
}

TOPDIR=$(dirname "$0")

# 位置パラメータへの設定
args=$(getopt -o r:s: -- "$@") || exit 1
eval "set -- $args"

while [ $# -gt 0 ]; do
  case "$1" in
    -r)
        OPT_N_RECEIVERS=$2;
        shift 2 ;;
    -s)
        OPT_N_SENDERS=$2;
        shift 2   ;;
    '--')
        shift;
        break ;;
  esac
done

# mandatory option check
if [ -z "$OPT_N_RECEIVERS" ] || [ -z "$OPT_N_SENDERS" ]; then
  exit 1
fi

RECEIVER="$TOPDIR/../client/m-receiver"
SENDER="$TOPDIR/../client/m-sender"
BROKER="$TOPDIR/../broker/target/release/raft-rs-broker"
BROKER_PID_FILE="$TOPDIR/../broker.pid"

FIRST_RECEIVER_ID=10000
FIRST_SENDER_ID=1
MESSAGE_COUNT=100000
BROKER_LEADER_IP="127.0.0.1:5555"
BROKER_ADDR="127.0.0.1:"
BROKER_PORT=5555
DEBUG_LEVEL=3
NUM_RECEIVERS="$OPT_N_RECEIVERS"
NUM_SENDERS="$OPT_N_SENDERS"

date=$(date +"%Y%m%d-%H%M%S")

# killall raft-rs-broker

BROKERS_IP=(
    "127.0.0.1:5555"
    "127.0.0.1:5556"
    "127.0.0.1:5557"
)
num_nodes=${#BROKERS_IP[@]}
for i in $(seq 2 $num_nodes)
do
    ip=$BROKER_ADDR$(expr $BROKER_PORT + $i - 1)
    echo "$BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE --raft-addrs "${BROKERS_IP[@]:0:$num_nodes}""
    $BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE --raft-addrs "${BROKERS_IP[@]:0:$num_nodes}"&
done
sleep 4

num_messages=$(expr $MESSAGE_COUNT / $NUM_RECEIVERS)
for i in $(seq 1 $NUM_RECEIVERS)
do
    myid=$(expr $FIRST_RECEIVER_ID + $i - 1)
    $RECEIVER -b $BROKER_LEADER_IP -u $myid -c $num_messages -d $DEBUG_LEVEL &
done
last_receiver_id=$myid
sleep 4

num_messages=$(expr $MESSAGE_COUNT / $NUM_RECEIVERS / $NUM_SENDERS)
for i in $(seq 1 $NUM_SENDERS)
do
    myid=$(expr $FIRST_SENDER_ID + $i - 1)
    $SENDER -b $BROKER_LEADER_IP -u $myid -c $num_messages -d $DEBUG_LEVEL $FIRST_RECEIVER_ID-$last_receiver_id &
done

while is_receiver_working
do
    sleep 1
done
sleep 4

killall raft-rs-broker
sleep 4
