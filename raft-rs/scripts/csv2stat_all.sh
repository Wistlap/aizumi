#!/usr/bin/env bash

LOG_DIR=$1
TOP_DIR=$(dirname $0)

for s in $( ls $LOG_DIR/*.log | grep -ve 'mpstat.log$')
  do
  echo "../stat/stat_from_csv.py $1/$s > $1/$(echo $s | sed -e "s/^\([0-9]\+-[0-9]\+-[0-9]\+\).*$/\1/").stat"
  $TOP_DIR/stat_from_csv.py $s > "$(echo $s | sed -e "s/^\([0-9]\+-[0-9]\+-[0-9]\+\).*$/\1/").stat"
  done
