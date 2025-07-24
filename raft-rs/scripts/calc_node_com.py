#!/usr/bin/env python

import pandas as pd
import argparse

clock = 2_800_000_000 #実験に使用した計算機のクロック周波数に応じて書き換える

def process_first_file(file_path, target_node_id):
    df = pd.read_csv(file_path)
    df['tsc'] = df['tsc'] / clock

    # 条件で絞り込み
    allowed_nodes = [1, target_node_id]
    filtered = df[
        (df['msg_type'].isin([104, 105])) &
        (df['node_id_from'].isin(allowed_nodes)) &
        (df['node_id_to'].isin(allowed_nodes))
    ]

    # 最小 tsc を取る
    filtered = filtered.loc[
        filtered.groupby(['id', 'msg_type'])['tsc'].idxmin()
    ].reset_index(drop=True)

    # 分割
    df_104 = filtered[filtered['msg_type'] == 104].set_index('id')
    df_105 = filtered[filtered['msg_type'] == 105].set_index('id')

    # 同じ id に対する tsc 差を計算
    merged = df_104[['tsc']].join(df_105[['tsc']], lsuffix='_104', rsuffix='_105')
    merged['comm_time'] = (merged['tsc_105'] - merged['tsc_104'])

    # 99パーセンタイルでフィルタ
    threshold = merged['comm_time'].quantile(0.99)
    filtered = merged[merged['comm_time'] <= threshold]
    print(f"File1 Filtered rows:\n {filtered}")

    return filtered['comm_time'].mean()

def process_second_file(file_path):
    df = pd.read_csv(file_path)
    df['tsc'] = df['tsc'] / clock

    # 条件で絞り込み
    filtered = df[(df['msg_type'].isin([104, 105])) & (df['total_msg_count'] == 4)]

    # 最小 tsc を取る
    filtered = filtered.loc[
        filtered.groupby(['id', 'msg_type'])['tsc'].idxmin()
    ].reset_index(drop=True)

    # 分割
    df_104 = filtered[filtered['msg_type'] == 104].set_index('id')
    df_105 = filtered[filtered['msg_type'] == 105].set_index('id')

    # 同じ id に対する tsc 差を計算
    merged = df_104[['tsc']].join(df_105[['tsc']], lsuffix='_104', rsuffix='_105')
    merged['comm_time'] = (merged['tsc_104'] - merged['tsc_105']).abs()

    # 99パーセンタイルでフィルタ
    threshold = merged['comm_time'].quantile(0.90)
    filtered = merged[merged['comm_time'] <= threshold]
    # with pd.option_context('display.max_rows', 1000):
    print(f"File2 Filtered rows:\n {filtered}")

    return filtered['comm_time'].mean()

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--file1', help='Path to first log file')
    parser.add_argument('--file2', help='Path to second log file')
    args = parser.parse_args()

    if args.file1:
        node = int(args.file2.split('-')[-1])
        avg1 = process_first_file(args.file1, node)

    if args.file2:
        avg2 = process_second_file(args.file2)

    print(f"Average communication: avg1: {avg1*1000:.3f} ms, avg2: {avg2*1000:.3f} ms, diff: {abs(avg1 - avg2)*1000:.3f} ms")

if __name__ == '__main__':
    main()
