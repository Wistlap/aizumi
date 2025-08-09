#!/usr/bin/env python
import pandas as pd
import numpy as np
import glob
import re
import argparse
from concurrent.futures import ProcessPoolExecutor
from matplotlib import pyplot as plt
import seaborn
import datetime

# msg_type に対して 'min' / 'max' / 'mediam' を指定
msg_type_config = {
    101: 'min',
    103: 'min',
    104: 'mediam',
    105: 'mediam',
    106: 'mediam',
    107: 'min',
    108: 'min',
    109: 'min',
}

# 差分計算用ペア
pairs = [(101, 103), (103, 106), (106, 109), (101, 109)]

# ファイル名から情報を抽出する正規表現
reg = re.compile(
    r'.(?P<broker>[a-zA-Z][-a-zA-Z]*)-(?P<sender>[0-9]+)-(?P<receiver>[0-9]+)-'
    r'(?P<option1>[0-9]+)-(?P<message>[0-9]+)-(?P<node>[0-9]+)-(?P<date>[0-9]+-[0-9]+)-formatted\.log$'
)

def select_value(vals, method, node):
    """指定された method に応じて値を選択"""
    if len(vals) == 0:
        return np.nan

    vals = sorted(vals)
    if method == 'min':
        return vals[0]
    elif method == 'max':
        return vals[-1]
    elif method == 'mediam':
        index = (node // 2) - 1
        # print("node:", node, "index:", index, "vals:", vals)
        if len(vals) > index:
            return vals[index]
        else:
            return np.nan
    else:
        return np.nan

def process_file(path):
    df = pd.read_csv(path, skipinitialspace=True)
    match = reg.search(path)
    node_str = match.group('node') if match else "1"
    node = int(node_str)

    df_result = []

    for id_, group in df.groupby("id", sort=False):
        row = {"id": id_}
        for msg_type, method in msg_type_config.items():
            col = f"tsc_{msg_type}"
            if col in group.columns:
                values = group[col].dropna().tolist()
                row[col] = select_value(values, method, node)
        df_result.append(row)

    final_df = pd.DataFrame(df_result)
    return node_str, final_df

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('dir', help='log dir path')
    args = parser.parse_args()

    paths = sorted(glob.glob(f'{args.dir}/*formatted.log'))

    print("table:raft processing time(ms)")
    print("\t↓nodes\\timing", end="")
    for start, end in pairs:
        print(f"\t{start}-{end}", end="")
    print()

    with ProcessPoolExecutor() as executor:
        results = list(executor.map(process_file, paths))

    hoge = []
    for node, final_df in results:
        print(f"\t{node}", end="")
        for s, e in pairs:
            col_s = f"tsc_{s}"
            col_e = f"tsc_{e}"
            if col_s in final_df.columns and col_e in final_df.columns:
                diff = final_df[col_e] - final_df[col_s]
                mean_val = diff.mean() if not diff.empty else None
                if col_s == "tsc_101" and col_e == "tsc_109":
                    hoge.append((node, mean_val))
            else:
                mean_val = None
            print(f"\t{mean_val:.3f}" if mean_val is not None else "\tN/A", end="")
        print()
    # x軸をnode, y軸をmean_valにして折れ線グラフを描画
    df_plot = pd.DataFrame(hoge, columns=["node", "mean"])
    df_plot["node"] = df_plot["node"].astype(int)
    df_plot = df_plot.sort_values("node")

    # 折れ線グラフの描画
    # plt.figure(figsize=(8, 5))
    seaborn.set_theme(style="whitegrid")
    splot = seaborn.lineplot(data=df_plot, x="node", y="mean", marker="o")
    plt.xticks(df_plot["node"].unique())
    plt.xlabel("Node")
    plt.ylabel("Raft processing time (ms)")
    plt.tight_layout()
    splot.get_figure()
    date = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    file_name = f'{date}.pdf'
    plt.savefig(file_name)
    plt.close()


if __name__ == "__main__":
    main()
