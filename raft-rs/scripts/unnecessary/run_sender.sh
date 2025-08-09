#!/usr/bin/env bash

TOPDIR=$(dirname "$0")

################################################################
## Main

SENDER="$TOPDIR/../m-sender"

FIRST_RECEIVER_ID=10000
FIRST_SENDER_ID=1

BROKER_PORT=$1
NUM_SENDERS=$2
NUM_RECEIVERS=$3
MESSAGE_COUNT=$4
DEBUG_LEVEL=$5

last_receiver_id=$(expr $FIRST_RECEIVER_ID + $NUM_RECEIVERS - 1)

################
# invoke senders

num_messages=$(expr $MESSAGE_COUNT / $NUM_RECEIVERS / $NUM_SENDERS)

for i in $(seq 1 $NUM_SENDERS)
do
  myid=$(expr $FIRST_SENDER_ID + $i - 1)
  $SENDER -b $BROKER_PORT -u $myid -c $num_messages -d $DEBUG_LEVEL $FIRST_RECEIVER_ID-$last_receiver_id &
done
