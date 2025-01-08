#!/usr/bin/env bash

# use this script in scripts directory

function is_receiver_working() {
    test $(ps | grep m-receiver | wc -l) != 0
}

TOPDIR=$(dirname "$0")

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

LOG_DIR_1="../log/1_clients_n_nodes"
LOG_DIR_10="../log/10_clients_n_nodes"
LOG_DIR_N="../log/n_clients_5_nodes"

date=$(date +"%Y%m%d-%H%M%S")

killall raft-rs-broker

rm -rf ../log/*
mkdir -p ../log
mkdir -p $LOG_DIR_1
mkdir -p $LOG_DIR_10
mkdir -p $LOG_DIR_N


# s1,r1,nods1,..,10
num_receivers=1
num_senders=1
for i in $(seq 1 10)
do
    echo
    echo
    echo "s1 r1 broker-nodes $i"
    log_file_is_exist=0
    for j in $(seq $i)
    do
        ip=$BROKER_ADDR$(expr $BROKER_PORT + $j - 1)
        log_file=$LOG_DIR_1/raf-rs-broker-$num_senders-$num_receivers-1-$MESSAGE_COUNT-$i-$date.log
        if [ $log_file_is_exist -eq 0 ]; then
            $BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE -l $log_file -g $i&
            log_file_is_exist=1
        else
            $BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE -g $i&
        fi
    done
    sleep 4

    num_messages=$(expr $MESSAGE_COUNT / $num_receivers)
    $RECEIVER -b $BROKER_LEADER_IP -u $FIRST_RECEIVER_ID -c $num_messages -d $DEBUG_LEVEL &
    sleep 4

    num_messages=$(expr $MESSAGE_COUNT / $num_receivers / $num_senders)
    $SENDER -b $BROKER_LEADER_IP -u $FIRST_SENDER_ID -c $num_messages -d $DEBUG_LEVEL $FIRST_RECEIVER_ID &

    while is_receiver_working
    do
        sleep 1
    done
    sleep 4

    killall raft-rs-broker
    sleep 4
done


# s10,r10,nods1,..,10
num_receivers=10
num_senders=10
for i in $(seq 1 10)
do
    echo
    echo
    echo "s10 r10 nodes $i"
    log_file_is_exist=0
    for j in $(seq $i)
    do
        ip=$BROKER_ADDR$(expr $BROKER_PORT + $j - 1)
        log_file=$LOG_DIR_10/raf-rs-broker-$num_senders-$num_receivers-1-$MESSAGE_COUNT-$i-$date.log
        if [ $log_file_is_exist -eq 0 ]; then
            $BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE -l $log_file -g $i&
            log_file_is_exist=1
        else
            $BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE -g $i&
        fi
    done
    sleep 4

    num_messages=$(expr $MESSAGE_COUNT / $num_receivers)
    for j in $(seq 1 $num_receivers)
    do
        myid=$(expr $FIRST_RECEIVER_ID + $j - 1)
        $RECEIVER -b $BROKER_LEADER_IP -u $myid -c $num_messages -d $DEBUG_LEVEL &
    done
    last_receiver_id=$myid
    sleep 4

    num_messages=$(expr $MESSAGE_COUNT / $num_receivers / $num_senders)
    for j in $(seq 1 $num_senders)
    do
        myid=$(expr $FIRST_SENDER_ID + $j - 1)
        $SENDER -b $BROKER_LEADER_IP -u $myid -c $num_messages -d $DEBUG_LEVEL $FIRST_RECEIVER_ID-$last_receiver_id &
    done

    while is_receiver_working
    do
        sleep 1
    done
    sleep 4

    killall raft-rs-broker
    sleep 4
done


# s1,2,4,10,r1,2,4,10,nods5
for i in 1 2 4 10
do
    for j in 1 2 4 10
    do
        echo
        echo
        echo "s$i r$j nodes 5"
        log_file_is_exist=0
        for k in $(seq 5)
        do
            ip=$BROKER_ADDR$(expr $BROKER_PORT + $k - 1)
            log_file=$LOG_DIR_N/raf-rs-broker-$i-$j-1-$MESSAGE_COUNT-5-$date.log
            if [ $log_file_is_exist -eq 0 ]; then
                $BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE -l $log_file -g 5&
                log_file_is_exist=1
            else
                $BROKER -b $ip -d $DEBUG_LEVEL -p $BROKER_PID_FILE -g 5&
            fi
        done
        sleep 4

        num_messages=$(expr $MESSAGE_COUNT / $j)
        for k in $(seq 1 $j)
        do
            myid=$(expr $FIRST_RECEIVER_ID + $k - 1)
            $RECEIVER -b $BROKER_LEADER_IP -u $myid -c $num_messages -d $DEBUG_LEVEL &
        done
        last_receiver_id=$myid
        sleep 4

        num_messages=$(expr $MESSAGE_COUNT / $j / $i)
        for k in $(seq 1 $i)
        do
            myid=$(expr $FIRST_SENDER_ID + $k - 1)
            $SENDER -b $BROKER_LEADER_IP -u $myid -c $num_messages -d $DEBUG_LEVEL $FIRST_RECEIVER_ID-$last_receiver_id &
        done

        while is_receiver_working
        do
            sleep 1
        done
        sleep 4

        killall raft-rs-broker
        sleep 4
    done
done

echo
./csv2stat_all.sh ../log/1_clients_n_nodes
sleep 1
echo
./csv2stat_all.sh ../log/10_clients_n_nodes
sleep 1
echo
./csv2stat_all.sh ../log/n_clients_5_nodes
