#!/usr/bin/env python

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
#   dirs: スタットファイルを含むログディレクトリ(複複数指定可)
#   -t  : タイムアウト方式で実行したブローカのログであることの宣言
#   -n  : スレッドプール方式で実行したブローカのログであることの宣言
#   -s  : 散布図を出力
#   -b  : 平均棒グラフを出力
#####

# 散布図の描画
def scatter_plot(df, df_c, file):
  splot = seaborn.swarmplot(x=df_c[0],y=df_c[2],hue=df_c[1],data=df, palette="Set1",  dodge=True)
  plt.ylim(0, max(df[df_c[2]])*1.1)
  splot.get_figure()
  plt.savefig(file)
  plt.close()

# 平均値の棒グラフ描画
def bar_plot(df, df_c, file):
  splot = seaborn.barplot(x=df_c[0],y=df_c[2],hue=df_c[1],data=df, palette="Set1", errorbar=None, dodge=True)
  plt.ylim(0, max(df[df_c[2]])*1.1)
  # 凡例の表示形式を変更する場合
  # plt.legend(loc="center", title=df_c[1], ncol=5, bbox_to_anchor=(.5, 1.1))
  splot.get_figure()
  plt.savefig(file, bbox_inches='tight')
  plt.close()

# スループットの計算とデータフレームの整形
def main(dirs, n, t, scatter, bar):
  log_dirs = dirs
  paths = list(flatten(glob.glob(f'{log_dir}/*.log.stat') for log_dir in log_dirs))
  reg = re.compile('.(?P<broker>m-broker[-a-zA-Z]*)-(?P<sender>[0-9]+)-(?P<receiver>[0-9]+)-(?P<option>[0-9]+)-(?P<message>[0-9]+)-(?P<date>[0-9]+-[0-9]+).log.stat$')
  table = []
  if n:
    legend = 'thread:broker'
  elif t:
    legend = 'timeout:broker'
  else:
    legend = ''
  dataframe_column = ['sender-receiver', legend, 'throughput(msg/sec)']

  i = 1
  # 各ログディレクトリ内の .log.stat ファイルからスループットを計算しリストに保存
  for path in paths:
    match = reg.search(path)
    broker = match.group('broker')
    sender_num = int(match.group('sender'))
    receiver_num = int(match.group('receiver'))
    broker_opt_num = int(match.group('option'))
    message_num = int(match.group('message'))
    with open(path, 'r') as f:
      reader = csv.reader(f)
      lines = list(reader)
      time = float(lines[1][2])
      throughput = message_num/time
      table.append([i, broker_opt_num, broker, throughput])
    i = i + 1
  table.sort()
  table = [[i, f'{broker_opt}:{broker}', throughput] for i, broker_opt, broker, throughput in table]
  # print(table)


  # データフレームの作成
  df = pd.DataFrame(data=table, columns=dataframe_column)
  # print(df)
  date = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
  file_name = f'{date}.pdf'

  # コマンドライン引数の応じたグラフの作成
  if scatter:
    scatter_plot(df, dataframe_column, file_name)
  elif bar:
    bar_plot(df, dataframe_column, file_name)

  # 使用したログディレクトリをメタデータとしてファイルに埋め込む
  src_pdf = pypdf.PdfReader(file_name)
  dst_pdf = pypdf.PdfWriter()
  dst_pdf.clone_reader_document_root(src_pdf)
  dst_pdf.add_metadata(src_pdf.metadata)
  dst_pdf.add_metadata({'/Dirs': str(dirs)})
  dst_pdf.write(file_name)
  # pdf = pypdf.PdfReader(file_name)
  # print('used dirs :', pdf.metadata['/Dirs'])


# コマンドライン引数の parse と main 関数の呼び出し
if __name__ == '__main__':
  parser = argparse.ArgumentParser()
  parser.add_argument('dirs', help='log dir paths', nargs='+')
  group_broker_opt = parser.add_mutually_exclusive_group(required=True)
  group_broker_opt.add_argument('-n', help="display table title as number of threads",
                     action="store_true")
  group_broker_opt.add_argument('-t', help="display table title as timeout",
                     action="store_true")
  group_plot = parser.add_mutually_exclusive_group(required=True)
  group_plot.add_argument('-s', '--scatter', help="draw scatter plot",
                     action="store_true")
  group_plot.add_argument('-b', '--bar', help="draw bar plot",
                     action="store_true")
  args = parser.parse_args()
  main(args.dirs, args.n, args.t, args.scatter, args.bar)
