#!/usr/bin/env bash

##########
# This script is used when testing Raft communication between
# different computers.
# Manually start the leader on one computer, A, and use this
# script on the other computer, B.
# The command line arguments specify the number of senders -s
# and the number of receivers -r.
##########


TOPDIR=$(dirname "$0")

# set default values
OPT_N_MESSAGES=100000

# 位置パラメータへの設定
args=$(getopt -o r:s:n:c: -- "$@") || exit 1
eval "set -- $args"

while [ $# -gt 0 ]; do
    case "$1" in
    -r)
        OPT_N_RECEIVERS=$2;
        shift 2   ;;
    -s)
        OPT_N_SENDERS=$2;
        shift 2   ;;
    -n)
        OPT_N_NODES=$2;
        shift 2   ;;
    -c)
        OPT_N_MESSAGES=$2;
        shift 2   ;;
    '--')
        shift;
        break ;;
    esac
done

# mandatory option check
if [ -z "$OPT_N_RECEIVERS" ] || [ -z "$OPT_N_SENDERS" ] || [ -z "$OPT_N_NODES" ]; then
    echo "Usage: $0 -r <number of receivers> -s <number of senders> -n <number of nodes> [-c <number of message count>]"
    echo "Example: $0 -r 10 -s 10 -n 3"
    echo "Example: $0 -r 40 -s 40 -n 3 -c 100000"
    echo "This script requires the number of receivers, senders, and nodes to be specified."
    exit 1
fi

BROKER="$TOPDIR/../broker/target/release/raft-rs-broker"
BROKER_PID_FILE="$TOPDIR/../broker.pid"

DEBUG_LEVEL=3
NUM_RECEIVERS="$OPT_N_RECEIVERS"
NUM_SENDERS="$OPT_N_SENDERS"
NUM_NODES="$OPT_N_NODES"
MESSAGE_COUNT="$OPT_N_MESSAGES"

date=$(date +"%Y%m%d-%H%M%S")
LOGDIR="$TOPDIR/../log"
LOGFILE="$LOGDIR/test/raft-rs-broker-$NUM_SENDERS-$NUM_RECEIVERS-1-$MESSAGE_COUNT-$NUM_NODES-$date.log"

mkdir -p "$LOGDIR"
mkdir -p "$LOGDIR/test"

killall raft-rs-broker
sleep 1

BROKERS_IP=(
    "xx.xx.xx.xx:10000"
    "xx.xx.xx.xx:10001"
    "xx.xx.xx.xx:10002"
)

touch $LOGFILE

num_nodes=${NUM_NODES}
ip="${BROKERS_IP[0]}"  # Get the IP address of the first broker
echo "$BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE -l $LOGFILE --raft-addrs "${BROKERS_IP[@]:0:$num_nodes}""
$BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE -l $LOGFILE --raft-addrs "${BROKERS_IP[@]:0:$num_nodes}"&
