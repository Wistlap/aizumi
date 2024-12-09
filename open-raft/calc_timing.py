#!/usr/bin/env python
import pandas as pd
import matplotlib.pyplot as plt

# ファイルを読み込む
i=3
file_path = f"timestamp-{i}.log"
df = pd.read_csv(file_path, header=0)  # ヘッダあり

# クロック周波数（例：2.5 GHz = 2.5 * 10^9 Hz）
clock_frequency = 1.9 * 10**9  # 必要に応じてクロック周波数を変更

# タイムスタンプをms単位に変換
df['tsc'] = df['tsc'] / clock_frequency * 10**3
# print(df)

# timingごとにデータをフィルタリングして列として再構成
df_pivot = (
    df[df['timing'].isin([0, 1, 2, 3, 4, 5, 6])]  # timingが1, 2, 4, 5のみを抽出
    .pivot(index='id', columns='timing', values='tsc')  # timingを列に変換
    .reset_index()  # idを通常の列に戻す
)

# 列名をわかりやすく変更
df_pivot.columns.name = None  # カラム名の階層を解除
df_pivot.rename(columns={0: 'tsc_timing_0', 1: 'tsc_timing_1', 2: 'tsc_timing_2', 3: 'tsc_timing_3', 4: 'tsc_timing_4', 5: 'tsc_timing_5', 6: 'tsc_timing_6'}, inplace=True)

# tsc_timing_1を基準に差分を計算
df_pivot['tsc_timing_1'] = df_pivot['tsc_timing_1'] - df_pivot['tsc_timing_0']
df_pivot['tsc_timing_2'] = df_pivot['tsc_timing_2'] - df_pivot['tsc_timing_0']
df_pivot['tsc_timing_3'] = df_pivot['tsc_timing_3'] - df_pivot['tsc_timing_0']
df_pivot['tsc_timing_4'] = df_pivot['tsc_timing_4'] - df_pivot['tsc_timing_0']
df_pivot['tsc_timing_5'] = df_pivot['tsc_timing_5'] - df_pivot['tsc_timing_0']
df_pivot['tsc_timing_6'] = df_pivot['tsc_timing_6'] - df_pivot['tsc_timing_0']
df_pivot['tsc_timing_0'] = df_pivot['tsc_timing_0'] - df_pivot['tsc_timing_0']

# 新しいデータフレームの表示
# print(df_pivot)
# 計算結果をCSVファイルに保存
output_path = f"formatted_data-{i}.log"
df_pivot.to_csv(output_path, index=False)  # インデックスを出力しない

# プロット用のデータを整形
plot_data = pd.melt(
    df_pivot,
    id_vars=['id'],
    value_vars=['tsc_timing_0', 'tsc_timing_1', 'tsc_timing_2', 'tsc_timing_3', 'tsc_timing_4', 'tsc_timing_5', 'tsc_timing_6'],
    var_name='timing',
    value_name='tsc'
)

# プロット
plt.figure(figsize=(10, 6))
for key, group in plot_data.groupby('id'):
    plt.plot(group['tsc'], [key] * len(group), 'o-', label=f"ID {key}")  # 横軸: tsc, 縦軸: id

# グラフ設定
plt.xlabel("Time (ms)")
plt.ylabel("ID")
plt.title(f"TSC Timing Intervals by ID (nodes {i})")
plt.legend(title="ID", loc='upper left', bbox_to_anchor=(1, 1))
plt.grid(True)
plt.tight_layout()
plt.savefig(f"timing-intervals-{i}.pdf")