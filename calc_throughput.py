#!/usr/bin/env python

import pandas as pd

# ログファイルの読み込み
file_path = "timestamp.log"
# ヘッダはid, msg_type, tscの3列
df = pd.read_csv(file_path)

# クロック周波数（例：2.5 GHz = 2.5 * 10^9 Hz）
clock_frequency = 1.9 * 10**9  # 必要に応じてクロック周波数を変更

# タイムスタンプを秒単位に変換
df['tsc'] = df['tsc'] / clock_frequency

# 各 id ごとに msg_type が MSG_PUSH_REQ と MSG_SEND_REQ のデータをフィルタリング
push_req_df = df[df['msg_type'] == "MSG_PUSH_REQ"]
send_req_df = df[df['msg_type'] == "MSG_SEND_REQ"]

# 最もtscの大きいMSG_PUSH_REQを取得
max_tsc_push = push_req_df['tsc'].max()

# 最もtscの小さいMSG_SEND_REQを取得
min_tsc_send = send_req_df['tsc'].min()

# メッセージ処理時間
msg_proc_time = max_tsc_push - min_tsc_send

# id ごとにデータを結合
merged_df = pd.merge(push_req_df[['id', 'tsc']], send_req_df[['id', 'tsc']], on='id', suffixes=('_push', '_send'))

# 各 id ごとに MSG_PUSH_REQ と MSG_SEND_REQ の tsc の差を計算
merged_df['tsc_diff'] = merged_df['tsc_push'] - merged_df['tsc_send']

# メッセージ数（ユニークな id の数）
num_messages = len(merged_df)

# 平均秒を計算
mean_seconds = merged_df['tsc_diff'].mean()

# スループットの計算（メッセージ数 / 平均秒）
throughput = num_messages / msg_proc_time

# 結果の出力
print(f"Number of messages: {num_messages}")
print(f"Average time queued (ms): {mean_seconds * 1000}")
print(f"Throughput (msgs/sec): {throughput}")
