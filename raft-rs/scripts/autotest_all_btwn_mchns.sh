#!/usr/bin/env bash

TOPDIR=$(dirname "$0")
A_SCRIPT_DIR="your_script_a_directory"
A_SCRIPT="./your_script_a.sh"
B_SCRIPT_DIR="your_script_b_directory"
B_SCRIPT="./your_script_b.sh"
B_HOST="username@remote_host"
REPEAT=10  # 何回繰り返すか

for ((i=1; i<=REPEAT; i++)); do
    echo "==== Round $i ===="

    # 1) A側スクリプト起動
    echo "A: $A_SCRIPT"
    (
        cd "$A_SCRIPT_DIR" || exit
        exec "$A_SCRIPT" "$i"
    ) &
    PID_A=$!
    sleep 1

    # 2) B側スクリプト開始（ssh で実行，終了するまで待つ）
    echo
    echo "B: $B_SCRIPT"
    ssh $B_HOST "cd '$B_SCRIPT_DIR' && exec '$B_SCRIPT' '$i'"

    # 3) B終了したら，A側を停止
    echo
    echo "Killing runA.sh..."
    kill "$PID_A"
    wait "$PID_A" 2>/dev/null

    echo "Round $i done"
    echo
    sleep 1
done

