#!/bin/bash

# transtest_n1.sh IP/PORT           TRANS_ID TRANS_PROCESS1 TRANS_PROCESS MPH/TRANSFERS
# transtest_n1.sh 192.168.1.10:5555 2001     3              3             5000/6000

function is_host_working() {
  test $(ps | grep m-host_com | wc -l) != 0
}

LOG2CSV="./log2csv.sh"
PLOT_MPSTAT="./plot_from_mpstat_log.py"
cores="all,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31"

mkdir log

# mpstat
mpstat -P ALL 1 > log/mpstat_1.log &
MPSTAT_PID=$!

# m-broker
./m-broker     -d 0 -u 5000 -b $1 -l log/log_broker.log &
STR_PID_LIST=$!

read -p "start node2 ans press any key: " DATA

# m-amhs_com
./m-amhs_com   -d 0 -u 3001 -b $1 -l log/log_amhs.log &
PID=$!
STR_PID_LIST="${STR_PID_LIST},${PID}"

# m-trans_ctl
for ((i=$2; i<$2 + $3; i++))
do
  ./m-trans_ctl -d 3 -u $i -b $1 &
  PID=$!
  STR_PID_LIST="${STR_PID_LIST},${PID}"
done

# m-host_com
./m-host_com   -d 3 -u 1001  -b $1 -x $4 -m $5 &
PID=$!
STR_PID_LIST="${STR_PID_LIST},${PID}"

pidstat -p ${STR_PID_LIST} 1 > log/pidstat_1.log &
PIDSTAT_PID=$!

while is_host_working
do
sleep 1
done
sleep 2

kill -s SIGINT $MPSTAT_PID
kill -s SIGINT $PIDSTAT_PID

$LOG2CSV log/mpstat.log
$PLOT_MPSTAT log/mpstat.log.csv log/mpstat.log $cores
