#!/usr/bin/python3

import pandas as pd
import matplotlib
#matplotlib.use('TkAgg')
#from matplotlib import pyplot as plt
import sys
import re

ele_name1 = 'id'
ele_name2 = 'msg_type'
ele_name3 = 't6-t5'
csv_file = sys.argv[1]
end=99999

with open(csv_file) as f:
    s = f.read()

with open(csv_file, mode='w') as f:
    f.write(re.sub('I, \[[0-9]+-[0-9]+-[0-9]+T[0-9]+:[0-9]+:[0-9]+.[0-9]+ #[0-9]+\]  INFO -- : ', '', s))

df_orig = pd.read_csv(csv_file, skipinitialspace=True, header=1)
df = df_orig.sort_values([ele_name1, ele_name2])
dfs = df[df[ele_name2] == 1].reset_index()
dfr = df[df[ele_name2] == 7].reset_index()
#print(dfs)
#print(dfr)
average = (dfr['tsc']-dfs['tsc']).mean();

#print(re.findall('\d+-\d+-\d+', csv_file)[0])
print("receiver, sender, sender2receiver")
print((dfr['tsc'][end]-dfr['tsc'][0]), end=', ')
print((dfs['tsc'][end]-dfs['tsc'][0]), end=', ')
print((dfr['tsc'][end]-dfs['tsc'][0]))
print(average)
