#!/bin/bash

# transtest_n1.sh IP/PORT           TRANS_ID TRANS_PROCESS1 TRANS_PROCESS MPH/TRANSFERS
# transtest_n1.sh 192.168.1.10:5555 2001     3              3             5000/6000

function is_host_working() {
  test $(ps | grep m-host_com | wc -l) != 0
}

TOPDIR=$(cd $(dirname "$0")/..; pwd)

usage() {
  echo "Usage: transtest.sh BROKER_HOST FIRST_TRANS_CTL_ID TRANS_CTL_NUM_PER_HOST MPH TRANSFERS"
  echo "       Example: ./transtest.sh localhost:3000 2001 3 5000 6000"
} >&2

BROKER_HOST=$1
FIRST_TRANS_CTL_ID=$2
TRANS_CTL_NUM_PER_HOST=$3
MPH=$4
TRANSFERS=$5
LOG2CSV="$TOPDIR/scripts/log2csv.sh"
PLOT_MPSTAT="$TOPDIR/scripts/plot_from_mpstat_log.py"
cores="all,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31"
SSH_KEY="$HOME/.ssh/snipe_rsa"
REMOTE_HOST="localhost"
TRANS_CTL_SCRIPT="$TOPDIR/scripts/transtest_n2.sh"
REMOTE_SCRIPT_OPT="$BROKER_HOST $FIRST_TRANS_CTL_ID $TRANS_CTL_NUM_PER_HOST $MPH/$TRANSFERS"
LOCAL_SCRIPT_OPT="$BROKER_HOST $(expr $FIRST_TRANS_CTL_ID + $TRANS_CTL_NUM_PER_HOST) $TRANS_CTL_NUM_PER_HOST $MPH/$TRANSFERS"

if [ $# -ne 5 ]; then
  usage
  exit 1
fi

if [ ! -d "$TOPDIR/log" ]; then
  mkdir "$TOPDIR/log"
fi

# mpstat
mpstat -P ALL 1 > "$TOPDIR/log/mpstat_1.log" &
MPSTAT_PID=$!

# m-broker
"$TOPDIR/m-broker"     -d 0 -u 5000 -b $BROKER_HOST -l "$TOPDIR/log/log_broker.log" &
STR_PID_LIST=$!

sleep 1

# remote m-trans_ctl
ssh -T -i $SSH_KEY $REMOTE_HOST "$TRANS_CTL_SCRIPT $REMOTE_SCRIPT_OPT" &
sleep 1

# m-amhs_com
"$TOPDIR/m-amhs_com"   -d 0 -u 3001 -b $BROKER_HOST -l "$TOPDIR/log/log_amhs.log" &
PID=$!
STR_PID_LIST="${STR_PID_LIST},${PID}"

# m-trans_ctl
$TRANS_CTL_SCRIPT $LOCAL_SCRIPT_OPT &
sleep 2

# m-host_com
"$TOPDIR/m-host_com"   -d 3 -u 1001  -b $BROKER_HOST -x $(expr $TRANS_CTL_NUM_PER_HOST "*" 2) -m "$MPH/$TRANSFERS" &
PID=$!
STR_PID_LIST="${STR_PID_LIST},${PID}"

pidstat -p ${STR_PID_LIST} 1 > "$TOPDIR/log/pidstat_1.log" &
PIDSTAT_PID=$!

while is_host_working
do
sleep 1
done
sleep 2

kill -s SIGINT $MPSTAT_PID
kill -s SIGINT $PIDSTAT_PID

$LOG2CSV "$TOPDIR/log/mpstat_1.log"
$PLOT_MPSTAT "$TOPDIR/log/mpstat_1.log.csv" "$TOPDIR/log/mpstat_1.log" $cores
