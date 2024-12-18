#!/usr/bin/env python

import csv
import glob
import re
from collections import defaultdict
import statistics
import argparse
import itertools
flatten=itertools.chain.from_iterable

#####
# メッセージブローカのログを整形した後のデータから，センダ数とレシーバ数の
# 組み合わせごとのスループットの平均をScrapbox のテーブル記法で出力する
# このスクリプトの引数には整形後のデータ(スタットファイル)を含むログディレクトリを指定する．
#
# 引数
#   dirs: スタットファイルを含むログディレクトリ(複複数指定可)
#   -t  : タイムアウト方式で実行したブローカのログであることの宣言
#   -n  : スレッドプール方式で実行したブローカのログであることの宣言
#   -s  : スループットの標準偏差も出力する．
#####

# Scrapbox のテーブル記法でスループットを出力
def scrapbox_from_2ddict(d, title, legend='\\', does_print_sd=False):
  print(f'table:{title}')
  if does_print_sd:
    print(f'\tmean ± sd (msg/s)')
  print(f'\t{legend}', end='')
  for column_header in sorted(next(iter(d.values())).keys()):
    print(f'\t{column_header}', end='')
  print()

  for row_header, row_data in sorted(d.items()):
    print(f'\t{row_header}', end='')
    for _column_header, value in sorted(row_data.items()):
      mean = int(statistics.mean(value))
      sd = int(statistics.pstdev(value))
      if does_print_sd:
        print(f'\t{mean} ± {sd}', end='')
      else:
        print(f'\t{mean}', end='')
    print()


# 指定したログディレクトリ内の .log.stat ファイルからスループットを計算し，dict に格納する
def main(dirs, sd, n, t):
  log_dirs = dirs
  paths = list(flatten(glob.glob(f'{log_dir}/*.log.stat') for log_dir in log_dirs))
  reg = re.compile('.*-(?P<sender>[0-9]+)-(?P<receiver>[0-9]+)-(?P<thread>[0-9]+)-(?P<message>[0-9]+)-(?P<node>[0-9]+)-(?P<date>[0-9]+-[0-9]+).log.stat$')
  tables = defaultdict(lambda: defaultdict(lambda: defaultdict(list)))

  for path in paths:
    match = reg.search(path)
    sender_num = int(match.group('sender'))
    receiver_num = int(match.group('receiver'))
    thread_num = int(match.group('thread'))
    message_num = int(match.group('message'))
    with open(path, 'r') as f:
      reader = csv.reader(f)
      lines = list(reader)
      time = float(lines[1][2])
      throughput = int(message_num/time)
      tables[thread_num][sender_num][receiver_num].append(throughput)
  if n:
    title = "thread"
  elif t:
    title = "timeout"
  else:
    title = ""

  for thread_num, table in sorted(tables.items()):
    scrapbox_from_2ddict(table,
                        f'{title}{thread_num}_novem_s2r',
                        legend='sender↓\\receiver→',
                        does_print_sd=sd)


# コマンドライン引数の parse と main 関数の呼び出し
if __name__ == '__main__':
  parser = argparse.ArgumentParser()
  parser.add_argument('-s', '--sd', help="output standard deviation",
                      action="store_true")
  parser.add_argument('dirs', help='log dir paths', nargs='+')
  group = parser.add_mutually_exclusive_group(required=True)
  group.add_argument('-n', help="display table title as number of threads",
                     action="store_true")
  group.add_argument('-t', help="display table title as timeout",
                     action="store_true")
  args = parser.parse_args()
  main(args.dirs, args.sd, args.n, args.t)
