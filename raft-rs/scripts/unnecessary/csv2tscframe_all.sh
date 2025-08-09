#!/usr/bin/env bash

LOG_DIR=$1
TOP_DIR=$(dirname $0)

for log in $((cd "$LOG_DIR"; ls -1 *.log) | grep -E '[a-zA-Z][-a-zA-Z]*+-[0-9]+-[0-9]+-[0-9]+-[0-9]+-[0-9]+-[0-9]+-[0-9]+\.log$')
do
  LOG_FILE="$LOG_DIR/$log"
  echo "$TOP_DIR/csv2tscframe.py $LOG_FILE"
  "$TOP_DIR/csv2tscframe.py" "$LOG_FILE"
done
