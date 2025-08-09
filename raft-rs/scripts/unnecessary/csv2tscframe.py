#!/usr/bin/env python
import pandas as pd
from functools import reduce
import argparse

parser = argparse.ArgumentParser()
parser.add_argument('dir', help='log dir path')
args = parser.parse_args()

# ログファイル読み込み
path = args.dir
df = pd.read_csv(path, skipinitialspace=True)
clock = 2800000000 # クロック周波数
df['tsc'] = (df['tsc'] / clock) * 1000 # ms 単位に変換
dfs = []

# 各 msg_type ごとに分けて、同じ id ごとに出現順インデックスを付ける
df["idx"] = df.groupby(["id", "msg_type"]).cumcount()

# msg_type ごとのデータフレームに分けて、列名を整えてから統合
dfs = []
for msg_type in sorted(df["msg_type"].unique()):
    temp = df[df["msg_type"] == msg_type][["id", "idx", "tsc"]].copy()
    temp = temp.rename(columns={"tsc": f"tsc_{msg_type}"})
    dfs.append(temp)

# 全部を outer join（on=["id", "idx"]）で結合
merged = reduce(lambda left, right: pd.merge(left, right, on=["id", "idx"], how="outer"), dfs)

# idx列削除とソート
merged = merged.drop(columns=["idx"])
merged = merged.sort_values(by="id")

# ファイル出力
filename = path.split('.log')[0]
output = f'{filename}-formatted.log'
merged.to_csv(output, index=False)
