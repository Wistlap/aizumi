#!/usr/bin/env python3
import pandas as pd
import argparse, os, re

clock = 2_800_000_000
reg = re.compile('.(?P<broker>[a-zA-Z][-a-zA-Z]*)-(?P<sender>[0-9]+)-(?P<receiver>[0-9]+)-(?P<option1>[0-9]+)-(?P<message>[0-9]+)-(?P<totalnode>[0-9]+)-(?P<date>[0-9]+-[0-9]+).log(?:-(?P<node>[0-9]+))?$')

def load_df(p):
    df = pd.read_csv(p)
    df['tsc'] = (df['tsc']/clock)*1000
    return df

parser = argparse.ArgumentParser()
parser.add_argument('--leader', required=True)
parser.add_argument('--followers', nargs='*')
args = parser.parse_args()

# leader load
L = load_df(args.leader)
L = L.sort_values(['id','msg_type','tsc'])

# ノード数1（フォロワなし）の場合
if not args.followers:
    L101 = L[L['msg_type'] == 101].reset_index()
    L109 = L[L['msg_type'] == 109].reset_index()

    out = pd.DataFrame()
    out['id'] = L101['id']
    out['T_ready'] = pd.NA
    out['T_fcpu'] = pd.NA
    out['T_com'] = pd.NA
    out['T_proc'] = pd.NA
    out['T'] = L109['tsc'] - L101['tsc']

    cols = ['T_ready','T_fcpu','T_com','T_proc','T']
    out = out[(out[['T']] >= 0).all(axis=1)]

    for c in cols[:-1]:
        print(f'{c} (ms)', end=', ')
    print(f'{cols[-1]} (ms)')

    for c in cols[:-1]:
        print(out[c].mean(), end=', ')
    print(out[cols[-1]].mean())
    exit(0)

# フォロワありの場合
# nノード推定→border
m = reg.search(os.path.basename(args.leader))
totalnode = int(m.group('totalnode')) if m else len(args.followers)+1
border = totalnode//2

# leader -> flatten: pivot per id
pivot = L.pivot_table(index='id', columns='msg_type', values='tsc', aggfunc='min').reset_index()
pivot = pivot.rename(columns={101:'ts101',104:'ts104',105:'ts105',109:'ts109'})

# id × border 個に展開 → leader105順の node_id_from を縦持ち
l105 = L[L['msg_type']==105].sort_values(['id','tsc'])[['id','node_id_from','tsc']]
l105['rank'] = l105.groupby('id').cumcount()+1
l105 = l105[l105['rank']<=border][['id','node_id_from']]
# join pivot×l105 to restrict pairs → (id,fnode) to use
l105 = l105.rename(columns={'node_id_from': 'fnode'})
pivot_expanded = pivot.merge(l105, on='id', how='inner')
# pivot_expanded has id,ts101,ts104,ts105,ts109,fnode
# print(pivot_expanded)

# follower読み込み→集約 table(id,fnode,f104,f105)
fl = []
for fp in args.followers:
    f = load_df(fp)
    f = f[f['msg_type'].isin([104,105])]
    mm = reg.search(os.path.basename(fp))
    node=int(mm.group('node')) if mm else None
    f['fnode']=node
    fl.append(f)
F = pd.concat(fl)
# min per id,fnode
# f104: 特定フォロワだけから id,fnode 単位の最小値
target_fnodes = pivot_expanded['fnode'].unique()
f104 = (
    F[(F['msg_type']==104) & (F['fnode'].isin(target_fnodes))]
    .groupby(['id','fnode'])['tsc'].min()
    .reset_index(name='f104')
)

# merge leader pivot_expanded × f104
merged = pivot_expanded.merge(f104, on=['id','fnode'], how='inner')

# f105: フォロワ全員から id 単位の最小値
f105 = F[F['msg_type']==105].groupby('id')['tsc'].min().reset_index(name='f105')
# f105 を id 単位で結合
merged = merged.merge(f105, on='id', how='left')
# print(merged)

# for each id： aggregate follower stats vectorized → groupby
def calc(group):
    ts101 = group['ts101'].iloc[0]
    ts104 = group['ts104'].iloc[0]
    ts105L= group['ts105'].iloc[0]
    ts109 = group['ts109'].iloc[0]
    min105 = group['f105'].min()
    max104 = group['f104'].max()
    return pd.Series({
        'T_ready': ts104-ts101,
        'T_fcpu': max104-min105,
        'T_com': (ts105L-ts104)-(max104-min105),
        'T_proc': ts109-ts105L,
        'T': ts109-ts101
    })

out = merged.groupby('id').apply(calc).reset_index()
# 時間カラム（負の値がNG）
cols = ['T_ready','T_fcpu','T_com','T_proc','T']
# 行単位で、いずれか <0 のものがあれば除外
out = out[(out[cols] >= 0).all(axis=1)]
# print(out)

# print("\n--Average(ms)--")
for c in cols[:-1]:
    print(f'{c} (ms)', end=', ')
print(f'{cols[-1]} (ms)')

for c in cols[:-1]:
    print(out[c].mean(), end=', ')
print(out[cols[-1]].mean())
