#!/usr/bin/env bash

function is_receiver_working() {
  test $(ps | grep m-receiver | wc -l) != 0
}

TOPDIR=$(dirname "$0")

################################################################
## Main

RECEIVER="$TOPDIR/../m-receiver"

FIRST_RECEIVER_ID=10000

BROKER_PORT=$1
NUM_RECEIVERS=$2
MESSAGE_COUNT=$3
DEBUG_LEVEL=$4

################
# invoke receivers

num_messages=$(expr $MESSAGE_COUNT / $NUM_RECEIVERS)

for i in $(seq 1 $NUM_RECEIVERS)
do
  myid=$(expr $FIRST_RECEIVER_ID + $i - 1)
  $RECEIVER -b $BROKER_PORT -u $myid -c $num_messages -d $DEBUG_LEVEL &
done

while is_receiver_working
do
    sleep 1
done
