#!/usr/bin/env python

import numpy as np
import pandas as pd
from matplotlib import pyplot as plt
import csv
import glob
import re
import argparse
import itertools
flatten=itertools.chain.from_iterable
import datetime
import seaborn
import pypdf

#####
# メッセージブローカのログを整形した後のデータからグラフを生成する．
# このスクリプトの引数には整形後のデータ(スタットファイル)を含むログディレクトリを指定する．
#
# 引数
#   dir : スタットファイルを含むログディレクトリ
#   -t  : タイムアウト方式で実行したブローカのログであることの宣言
#   -n  : スレッドプール方式で実行したブローカのログであることの宣言
#   -b  : 平均累積棒グラフを出力
#   -l  : 折れ線グラフを出力
#####



# 平均値の累積棒グラフ描画
def bar_plot(df, df_c, file):
  # s-r と node を1つの複合キーに
  df['sr-node'] = df[df_c[1]] + '-' + df[df_c[0]]
  df = df[df[df_c[2]] != 'T']
  # print(df.head(10))

  # ピボットで T ごとの値を列に
  pivot = df.pivot_table(index='node', columns=df_c[2], values=df_c[3], fill_value=0)
  # print(pivot)
  fig, ax = plt.subplots(figsize=(6,4))
  bottom = np.zeros(len(pivot))
  colors = plt.cm.tab10.colors
  hatches = ['//', '\\\\', '||', 'xx']  # ハッチパターンのリスト

  for i, col in enumerate(pivot.columns):
    ax.bar(pivot.index, pivot[col], bottom=bottom, label=col, hatch=hatches[i], color=colors[i], alpha=0.7)
    bottom += pivot[col]

    # sr-node を node と s-r に分けて x軸の見た目を調整
    ax.set_xticks(range(len(pivot)))
    # ax.set_xticklabels(pivot.index, rotation=45, ha='right')

    ax.set_xlabel('node')
    ax.set_ylabel(df_c[3])
    ax.legend(title=df_c[2], loc='upper center', bbox_to_anchor=(0.5, 1.2), ncol=len(pivot.columns))  # 凡例をグラフの上に配置
    # 凡例を横並びにする
  plt.savefig(file, bbox_inches='tight')
  pivot['calcT'] = pivot['T_ready'] + pivot['T_fpcu'] + pivot['T_com'] + pivot['T_proc']
  print(pivot)

# 折れ線グラフ描画
def line_plot(df, df_c, file):
  target_section = 'T'
  df = df[df[df_c[2]]==target_section]  # 0以下の値を除外
  splot = seaborn.lineplot(x=df_c[0], y=df_c[3], hue=df_c[1], data=df, palette="Set1", marker='o')
  # グリッドを追加
  plt.grid(axis='y', alpha = 0.5)
  plt.ylim(0, max(df[df_c[3]])*1.1)
  splot.get_figure()
  plt.savefig(file, bbox_inches='tight')
  plt.close()

# スループットの計算とデータフレームの整形
def main(dir, bar, line):
  paths = list(glob.glob(f'{dir}/*.log.raft-stat'))
  reg = re.compile('.(?P<broker>[a-zA-Z][-a-zA-Z]*)-(?P<sender>[0-9]+)-(?P<receiver>[0-9]+)-(?P<option1>[0-9]+)-(?P<message>[0-9]+)-(?P<node>[0-9]+)-(?P<date>[0-9]+-[0-9]+).log.raft-stat$')

  legend = 'sender-receiver'
  dataframe_column = ['node', legend, 'T section', 'proc time (ms)']

  table = []
  # 各ログディレクトリ内の .log.stat ファイルからスループットを計算しリストに保存
  for path in paths:
    match = reg.search(path)
    # broker = match.group('broker')
    sender_num = int(match.group('sender'))
    receiver_num = int(match.group('receiver'))
    node_num = int(match.group('node'))
    # message_num = int(match.group('message'))
    with open(path, 'r') as f:
      reader = csv.reader(f)
      lines = list(reader)
      t_ready = float(lines[1][0]) if lines[1][0] else 0.0
      t_fpcu = float(lines[1][1]) if lines[1][1] else 0.0
      t_com = float(lines[1][2]) if lines[1][2] else 0.0
      t_proc = float(lines[1][3]) if lines[1][3] else 0.0
      t = float(lines[1][4]) if lines[1][4] else 0.0
      table.append([node_num, (sender_num, receiver_num), [t_ready, t_fpcu, t_com, t_proc, t]])
      # table.append([node_num, (sender_num, receiver_num), t])
  table.sort()
  # print(table)

  types = ['T_ready', 'T_fpcu', 'T_com', 'T_proc', 'T']
  result = []
  for (n, c, ts) in table:
    for j, t in enumerate(ts):
      result.append([f'{n}', f'{c[0]}-{c[1]}', types[j], t])
  # print(table)
  # print(result)


  # # データフレームの作成
  df = pd.DataFrame(result, columns=dataframe_column)
  # print(df)

  date = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
  file_name = f'{date}.pdf'

  # # コマンドライン引数の応じたグラフの作成
  if bar:
    bar_plot(df, dataframe_column, file_name)
  elif line:
    line_plot(df, dataframe_column, file_name)

  # # 使用したログディレクトリをメタデータとしてファイルに埋め込む
  # src_pdf = pypdf.PdfReader(file_name)
  # dst_pdf = pypdf.PdfWriter()
  # dst_pdf.clone_reader_document_root(src_pdf)
  # dst_pdf.add_metadata(src_pdf.metadata)
  # dst_pdf.add_metadata({'/Dirs': str(dirs)})
  # dst_pdf.write(file_name)
  # pdf = pypdf.PdfReader(file_name)
  # print('used dirs :', pdf.metadata['/Dirs'])


# コマンドライン引数の parse と main 関数の呼び出し
if __name__ == '__main__':
  parser = argparse.ArgumentParser()
  parser.add_argument('dir', help='log dir path')
  group_plot = parser.add_mutually_exclusive_group(required=True)
  group_plot.add_argument('-b', '--bar', help='draw "Stacked" bar plot',
                    action="store_true")
  group_plot.add_argument('-l', '--line', help="draw line plot",
                    action="store_true")
  args = parser.parse_args()
  main(args.dir, args.bar, args.line)
