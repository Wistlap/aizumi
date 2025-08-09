#!/bin/bash

# transtest_n2.sh IP/PORT           TRANS_ID TRANS_PROCESS2
# transtest_n2.sh 192.168.1.10:5555 2001     3

function is_trans_working() {
  test $(ps | grep m-trans_ctl | wc -l) != 0
}

TOPDIR=$(cd $(dirname "$0")/..; pwd)
LOG2CSV="$TOPDIR/scripts/log2csv.sh"
PLOT_MPSTAT="$TOPDIR/scripts/plot_from_mpstat_log.py"
cores="all,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31"
cd $TOPDIR

if [ ! -d "$TOPDIR/log" ]; then
  mkdir "$TOPDIR/log"
fi

#mpstat -P ALL 1 > "$TOPDIR/log/mpstat_n2.log" &
#MPSTAT_PID=$!

# m-trans_ctl
for ((i=$2; i<$2 + $3; i++))
do
  echo "$TOPDIR/m-trans_ctl -d 0 -u $i -b $1"
  "$TOPDIR/m-trans_ctl" -d 3 -u $i -b $1 &
  PID=$!
  STR_PID_LIST="${STR_PID_LIST},${PID}"
done



#pidstat -p ${STR_PID_LIST} 1 > "$TOPDIR/log/pidstat_2.log" &
#PIDSTAT_PID=$!

while is_trans_working
do
sleep 1
done
#sleep 2
#
#kill -s SIGINT $MPSTAT_PID
#kill -s SIGINT $PIDSTAT_PID
#
#$LOG2CSV "$TOPDIR/log/mpstat_2.log"
#$PLOT_MPSTAT "$TOPDIR/log/mpstat_2.log.csv" "$TOPDIR/log/mpstat_2.log" $cores
