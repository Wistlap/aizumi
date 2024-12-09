#!/usr/bin/env bash

function is_receiver_working() {
    test $(ssh  ps | grep m-recei\
ver | wc -l) != 0
}

function dns() {
  case "$1" in
    "novem" ) echo "172.23.81.23:3333";;
    "snipe" ) echo "172.23.81.25:3333";;
    "radial" ) echo "172.23.81.26:3333";;
    * ) usage exit 1;;
  esac
}

function add2ssh() {
  case "$1" in
    "novem" ) echo "nakagawa@novem.swlab.cs.okayama-u.ac.jp ";;
    "snipe" ) echo "nakagawa@snipe.swlab.cs.okayama-u.ac.jp" ;;
    "radial" ) echo "nakagawa@radial.swlab.cs.okayama-u.ac.jp ";;
    * ) usage exit 1;;
  esac
}
TOPDIR=$(pwd)

################################################################
## Parse option

usage() {
  echo "Usage: autotest.sh [-c N-MESSAGES] [-d LEVEL] [-l LOG-FILE] [-m] {-n N-THREADS | -t TIMEOUT} -r N-RCV -s N-SND -a A-SND -b A-BRO -v A-RCV"
  echo "  -h            Show this message"
  echo "  -c N-MESSAGES Number of messages (default:100000)"
  echo "  -d LEVEL      Set LOG level (default:2)"
  echo "  -l LOG-FILE   Set broker's log file (default: stdout)"
  echo "  -m            Use mpstat and make graph"
  echo "  -n N-THREADS  Use m-broker-tp and set number of threads"
  echo "  -t TIMEOUT    Use m-broker and set epoll_wait timeout in milliseconds"
  echo "  -r N-RCV      Number of receiver processes"
  echo "  -s N-SND      Number of sender processes"
  echo "  -a A-SND      Address of sender processes (default: novem)"
  echo "  -b A-BRO      Address of broker processes (default: novem)"
  echo "  -v A-RCV      Address of receiver processes (default: novem)"
} >&2

# set default values
OPT_DEBUG_LEVEL=2
OPT_LOG_FILE="stdout"
OPT_MPSTAT_FLAG=false
OPT_N_MESSAGES=100000
OPT_SEND_ADDRESS="novem"
OPT_BRO_ADDRESS="novem"
OPT_RECV_ADDRESS="novem"

while getopts "c:d:hl:mn:r:s:t:a:b:v:" flag
do
  case $flag in
    # getopts sets '?' to flag on error.
    \?|h) OPT_ERROR=1 ;;
    c)    OPT_N_MESSAGES="$OPTARG"    ;;
    d)    OPT_DEBUG_LEVEL="$OPTARG"   ;;
    l)    OPT_LOG_FILE="$OPTARG"      ;;
    m)    OPT_MPSTAT_FLAG=true        ;;
    n)    OPT_THREADS_NUM="$OPTARG"   ;;
    r)    OPT_N_RECEIVERS="$OPTARG"   ;;
    s)    OPT_N_SENDERS="$OPTARG"     ;;
    t)    OPT_TIMEOUT="$OPTARG"       ;;
    a)   OPT_SEND_ADDRESS="$OPTARG"  ;;
    b)   OPT_BRO_ADDRESS="$OPTARG"   ;;
    v)   OPT_RECV_ADDRESS="$OPTARG"  ;;
  esac
done
shift $(( $OPTIND - 1 ))

# unknown option check
if [ "$OPT_ERROR" = 1 -o $# -ne 0 ]; then
  usage
  exit 1
fi

# mandatory option check
if [ -z "$OPT_N_RECEIVERS" -o -z "$OPT_N_SENDERS" ]; then
  usage
  exit 1
fi

################################################################
## Main

RUN_RECEIVER="$TOPDIR/run_receiver.sh"
RUN_SENDER="$TOPDIR/run_sender.sh"
LOG2CSV="$TOPDIR/log2csv.sh"
PLOT_MPSTAT="$TOPDIR/plot_from_mpstat_log.py"
BROKER_PID_FILE="$TOPDIR/../broker.pid"
MPSTAT_LOGDIR="$TOPDIR/../mpstat_log"

FIRST_RECEIVER_ID=10000
FIRST_SENDER_ID=1
MESSAGE_COUNT=$OPT_N_MESSAGES

DEBUG_LEVEL="$OPT_DEBUG_LEVEL"
NUM_SENDERS="$OPT_N_SENDERS"
NUM_RECEIVERS="$OPT_N_RECEIVERS"
LOG_FILE="$OPT_LOG_FILE"
TIMEOUT="$OPT_TIMEOUT"
THREADS_NUM="$OPT_THREADS_NUM"
BROKER_PORT="localhost:3333"
date=$(date +"%Y%m%d-%H%M%S")
cores="all,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31"

################
# cleanup garbages and update binaries

if [ -f "$BROKER_PID_FILE" ]; then
  kill $(cat "$BROKER_PID_FILE")
  sleep 1
  rm -f "$BROKER_PID_FILE"
fi

(cd $TOPDIR/..; make)

################
# invoke mpstat

MPSTAT_LOGFILE="$NUM_SENDERS-$NUM_RECEIVERS-$THREADS_NUM-$MESSAGE_COUNT-$date"
if "${OPT_MPSTAT_FLAG}"; then
  test -d $MPSTAT_LOGDIR || mkdir $MPSTAT_LOGDIR
  mpstat -P ALL 1 > $MPSTAT_LOGDIR/$MPSTAT_LOGFILE.log &
  MPSTAT_PID=$!
fi

################
# invoke m-broker
BROKER_PORT=$(dns $OPT_BRO_ADDRESS)
# select timeout or thread
if [ -z "$OPT_TIMEOUT" -a -z "$OPT_THREADS_NUM" ]; then
  usage
  exit 1
elif [ -n "$OPT_TIMEOUT" ]; then
  ruby ../m-broker-rb/m-broker.rb -t $OPT_TIMEOUT -b $BROKER_PORT -d 1 -p $BROKER_PID_FILE -l "$LOG_FILE" -c 99999 -u 0 &
else
  ruby ../m-broker-rb/m-broker-tp.rb -t $OPT_THREADS_NUM -b $BROKER_PORT -d 1 -p $BROKER_PID_FILE -l "$LOG_FILE" -c 99999 -u 0 &
fi
sleep 1

################
# invoke receivers

if [ "$OPT_RECV_ADDRESS" = "$OPT_BRO_ADDRESS" ]; then
    $RUN_RECEIVER $BROKER_PORT $NUM_RECEIVERS $MESSAGE_COUNT $DEBUG_LEVEL &
    RECV_ID=$!
elif [ "$OPT_RECV_ADDRESS" != "$OPT_BRO_ADDRESS" ]; then
    SSH_ADD=$(add2ssh $OPT_RECV_ADDRESS)
    ssh $SSH_ADD $RUN_RECEIVER $BROKER_PORT $NUM_RECEIVERS $MESSAGE_COUNT $DEBUG_LEVEL &
    RECV_ID=$!
fi

sleep 3

################
# invoke senders
if [ "$OPT_SEND_ADDRESS" = "$OPT_BRO_ADDRESS" ]; then
    $RUN_SENDER $BROKER_PORT $NUM_SENDERS $NUM_RECEIVERS $MESSAGE_COUNT $DEBUG_LEVEL &
elif [ "$OPT_SEND_ADDRESS" != "$OPT_BRO_ADDRESS" ]; then
    SSH_ADD=$(add2ssh $OPT_SEND_ADDRESS)
    ssh $SSH_ADD $RUN_SENDER $BROKER_PORT $NUM_SENDERS $NUM_RECEIVERS $MESSAGE_COUNT $DEBUG_LEVEL &
fi

wait ${RECV_ID[@]}

$TOPDIR/../m-sender -b $BROKER_PORT -u 0 -c $THREADS_NUM -d 2 5000 -s 1

sleep 5

################
# make graph

if "${OPT_MPSTAT_FLAG}"; then
  while is_receiver_working
  do
    sleep 1
  done
  sleep 2

  kill -s SIGINT $MPSTAT_PID

  $LOG2CSV $MPSTAT_LOGDIR/$MPSTAT_LOGFILE.log
  $PLOT_MPSTAT $MPSTAT_LOGDIR/$MPSTAT_LOGFILE.log.csv $MPSTAT_LOGDIR/$MPSTAT_LOGFILE $cores

fi
