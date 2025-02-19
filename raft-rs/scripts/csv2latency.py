#!/usr/bin/env python

import pandas as pd
import matplotlib
#matplotlib.use('TkAgg')
from matplotlib import pyplot as plt
import sys
import re

ele_name1 = 'id'
ele_name2 = 'msg_type'
ele_name3 = 't6-t5'
clock = 3400000000
csv_file = sys.argv[1]
df_orig = pd.read_csv(csv_file, skipinitialspace=True)
df = df_orig.sort_values([ele_name1, ele_name2])
dfs = df[df[ele_name2] == 1].reset_index()
dfr = df[df[ele_name2] == 7].reset_index()
average = (dfr['tsc']-dfs['tsc']).mean();

df = ((dfr['tsc']-dfs['tsc'])*1000)/clock
plt.hist(df)
plt.xlabel('time(ms)')
plt.ylabel('frequency')
plt.savefig("test.pdf")
print(df)
print("average")
print(average/clock)
