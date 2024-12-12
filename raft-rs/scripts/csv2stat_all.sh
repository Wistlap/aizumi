#!/usr/bin/env bash

LOG_DIR=$1
TOP_DIR=$(dirname $0)

for log in $((cd "$LOG_DIR"; ls -1 *.log) | grep -ve 'mpstat.log$')
do
  STAT_FILE="$LOG_DIR/$log.stat"
  LOG_FILE="$LOG_DIR/$log"
  echo "$TOP_DIR/stat_from_csv.py $LOG_FILE > $STAT_FILE"
  "$TOP_DIR/stat_from_csv.py" "$LOG_FILE" > "$STAT_FILE"
done
