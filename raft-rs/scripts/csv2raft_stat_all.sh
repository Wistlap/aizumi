#!/usr/bin/env bash

LOG_DIR=$1
TOP_DIR=$(dirname $0)

declare -A seen

# cd "$LOG_DIR" || exit 1

for log in $(cd "$LOG_DIR"; ls -1 *.log* | grep -ve 'stat$')
do
  # brokername-s-r-t-c-n を取り出す
  # 最後の "-YYYYMMDD-hhmmss.log" または ".log-[番号]" を取り除く
  prefix=$(echo "$log" | sed -E 's/-[0-9]{8}-[0-9]{6}\.log.*$//')

  # すでに処理済みの prefix はスキップ
  if [[ ${seen["$prefix"]} ]]; then
      continue
  fi
  seen["$prefix"]=1

  # 同じ prefix のファイルをすべて列挙
  group=( $(cd "$LOG_DIR"; ls "${prefix}"-[0-9][0-9][0-9][0-9][0-9][0-9][0-9][0-9]-[0-9][0-9][0-9][0-9][0-9][0-9].log* | grep -ve 'stat$') )

  leader="${group[0]}"
  followers=("${group[@]:1}")
  STAT_FILE="$LOG_DIR/${leader}.raft-stat"

  echo "$TOP_DIR/raft_stat_from_csv.py --leader "$LOG_DIR/$leader" --followers "${followers[@]/#/$LOG_DIR/}" > "$STAT_FILE""
  $TOP_DIR/raft_stat_from_csv.py --leader "$LOG_DIR/$leader" --followers "${followers[@]/#/$LOG_DIR/}" > "$STAT_FILE"
done
