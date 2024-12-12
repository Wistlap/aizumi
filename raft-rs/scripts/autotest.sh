#!/usr/bin/env bash

function is_receiver_working() {
  test $(ps | grep m-receiver | wc -l) != 0
}

TOPDIR=$(dirname "$0")

################################################################
## Parse option

usage() {
  echo "Usage: autotest.sh -b BROKER -r N -s N [-c N] [-d LEVEL] [-l LOG-FILE] [--replica N] [--runtime RUNTIME] [-- -BROKER-OPT...]"
  echo "  -h            Show this message"
  echo "  -b BROKER     Broker path"
  echo "  -c N          Number of messages (default:100000)"
  echo "  -d LEVEL      Set LOG level (default:2)"
  echo "  -l LOG-FILE   Set broker's log file (default: stdout) and Use mpstat and make graph"
  echo "  -r N          Number of receiver processes"
  echo "  -s N          Number of sender processes"
  echo " --replica      Number of replica processes"
  echo " --runtime      Runtime name"
  echo "  -BROKER-OPT   Set the option that depend on the broker used"
} >&2

# set default values
OPT_N_MESSAGES=100000
OPT_DEBUG_LEVEL=2
OPT_LOG_FILE="stdout"
OPT_LOG_FLAG=false
OPT_N_REPLICAS=0
OPT_RUNTIME=""

# 位置パラメータへの設定
args=$(getopt -o b:c:d:hl:r:s: --long replica:,runtime:: -- "$@") || exit 1
eval "set -- $args"

while [ $# -gt 0 ]; do
  case "$1" in
    -h)
        OPT_ERROR=1;
        break ;;
    -b)
        OPT_BROKER=$2;
        shift 2      ;;
    -c)
        OPT_N_MESSAGES=$2;
        shift 2 ;;
    -d)
        OPT_DEBUG_LEVEL=$2;
        shift 2 ;;
    -l)
        OPT_LOG_FILE=$2;
        OPT_LOG_FLAG=true;
        shift 2        ;;
    -r)
        OPT_N_RECEIVERS=$2;
        shift 2 ;;
    -s)
        OPT_N_SENDERS=$2;
        shift 2   ;;
    '--replica')
        OPT_N_REPLICAS=$2;
        shift 2  ;;
    '--runtime')
        OPT_RUNTIME="$2";
        shift 2 ;;
    '--')
        shift;
        break ;;
  esac
done

# unknown option check
if [ "$OPT_ERROR" = 1 ]; then
  usage
  exit 1
fi

# mandatory option check
if [ -z "$OPT_N_RECEIVERS" ] || [ -z "$OPT_N_SENDERS" ]; then
  usage
  exit 1
fi


################################################################
## Main

RECEIVER="$TOPDIR/../client/m-receiver"
SENDER="$TOPDIR/../client/m-sender"
LOG2CSV="$TOPDIR/log2csv.sh"
PLOT_MPSTAT="$TOPDIR/plot_from_mpstat_log.py"
BROKER_PID_FILE="$TOPDIR/../broker.pid"

FIRST_RECEIVER_ID=10000
FIRST_SENDER_ID=1

# Set the options in variables
BROKER="$OPT_BROKER"
MESSAGE_COUNT=$OPT_N_MESSAGES
DEBUG_LEVEL="$OPT_DEBUG_LEVEL"
LOG_FILE="$OPT_LOG_FILE"
NUM_RECEIVERS="$OPT_N_RECEIVERS"
NUM_SENDERS="$OPT_N_SENDERS"
BROKER_OPT="$*"
NUM_REPLICAS="$OPT_N_REPLICAS"
RUNTIME="$OPT_RUNTIME"
RUNTIME_PID_FILE="$TOPDIR/../runtime.pid"

BROKER_PORT="localhost:5555"
date=$(date +"%Y%m%d-%H%M%S")
cores="all,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31"

# Extract log directory and log file name without extension
LOG_DIR="$(dirname $LOG_FILE)"
LOG_FILE_NAME="$(basename $LOG_FILE .log)"

# For replication
CAPTURE="$TOPDIR/../replication/target/release/replication"
CAPTURE_PID_FILE="$TOPDIR/../capture.pid"

################
# cleanup garbages and update binaries

if [ -f "$BROKER_PID_FILE" ]; then
  kill $(cat "$BROKER_PID_FILE")
  sleep 1
  rm -f "$BROKER_PID_FILE"
fi

if [ -f "$CAPTURE_PID_FILE" ]; then
  while read line
  do
    kill $line
  done < "$CAPTURE_PID_FILE"
    sleep 1
    rm -f "$CAPTURE_PID_FILE"
fi

if [ -f "$RUNTIME_PID_FILE" ]; then
  kill $(cat "$RUNTIME_PID_FILE")
  sleep 1
  rm -f "$RUNTIME_PID_FILE"
fi

# (cd $TOPDIR/..; make)

################
# invoke mpstat

if "${OPT_LOG_FLAG}"; then
  test -d $LOG_DIR || mkdir $LOG_DIR &
  mpstat -P ALL 1 > $LOG_DIR/$LOG_FILE_NAME-mpstat.log &
  MPSTAT_PID=$!
fi


################
# create log file about runtime configration

CONFIG_LOGFILE="$LOG_DIR/$LOG_FILE_NAME.config"
echo "date: $date" > $CONFIG_LOGFILE
echo "broker path: $BROKER" >> $CONFIG_LOGFILE
echo "number of sender processes: $NUM_SENDERS" >> $CONFIG_LOGFILE
echo "number of receiver processes: $NUM_RECEIVERS" >> $CONFIG_LOGFILE
echo "Number of messages (default:100000): $MESSAGE_COUNT" >> $CONFIG_LOGFILE
echo "LOG level (default:2): $DEBUG_LEVEL" >> $CONFIG_LOGFILE
echo "Broker option: $BROKER_OPT" >> $CONFIG_LOGFILE

################
# invoke replication
if [ $NUM_REPLICAS -gt 0 ]; then
    NUM_REPLICAS=`expr $NUM_REPLICAS - 1`
    for i in $(seq 0 $NUM_REPLICAS )
    do
        address="127.0.0.1:3000$i"
        $CAPTURE $address &
        echo $! >> $CAPTURE_PID_FILE
        sleep 1
    done
fi

################
# invoke m-broker

case $RUNTIME in
    "" ) $BROKER -b $BROKER_PORT -d $DEBUG_LEVEL -p $BROKER_PID_FILE -l $LOG_FILE $BROKER_OPT& > "broker.log"
         sleep 1 ;;
    "wasmer" ) $RUNTIME --net $BROKER -- -b $BROKER_PORT -d $DEBUG_LEVEL -p $BROKER_PID_FILE -l $LOG_FILE $BROKER_OPT&
               echo $! >> $RUNTIME_PID_FILE
               sleep 1 ;;
esac
sleep 3

################
# invoke receivers

num_messages=$(expr $MESSAGE_COUNT / $NUM_RECEIVERS)

for i in $(seq 1 $NUM_RECEIVERS)
do
  myid=$(expr $FIRST_RECEIVER_ID + $i - 1)
  $RECEIVER -b $BROKER_PORT -u $myid -c $num_messages -d $DEBUG_LEVEL &
done

last_receiver_id=$myid
sleep 4

################
# invoke senders

num_messages=$(expr $MESSAGE_COUNT / $NUM_RECEIVERS / $NUM_SENDERS)

for i in $(seq 1 $NUM_SENDERS)
do
  myid=$(expr $FIRST_SENDER_ID + $i - 1)
  $SENDER -b $BROKER_PORT -u $myid -c $num_messages -d $DEBUG_LEVEL $FIRST_RECEIVER_ID-$last_receiver_id &
done

################
# make graph

if "${OPT_LOG_FLAG}"; then
  while is_receiver_working
  do
    sleep 1
  done
  sleep 4

  kill -s SIGINT $MPSTAT_PID

  $LOG2CSV $LOG_DIR/$LOG_FILE_NAME-mpstat.log
  $PLOT_MPSTAT $LOG_DIR/$LOG_FILE_NAME-mpstat.tsv $LOG_DIR/$LOG_FILE_NAME-mpstat $cores
fi
