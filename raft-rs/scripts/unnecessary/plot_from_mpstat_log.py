#!/usr/bin/env python

import pandas as pd
import matplotlib
from matplotlib import pyplot as plt
import sys
import re
import os
from matplotlib.backends.backend_pdf import PdfPages
pp = PdfPages('{}.pdf'.format(sys.argv[2]))

ele_name1 = 'cpu'
ele_name2 = 'time'
ele_name3 = 'usr'
ele_name4 = 'sys'
ele_name5 = 'soft'
csv_file = sys.argv[1]
df_orig = pd.read_csv(csv_file, skipinitialspace=True, sep='\s+')
df = df_orig.sort_values([ele_name1, ele_name2])

for i in sys.argv[3].split(','):
  index=list(range(len(df[df[ele_name1] == i])))
  sum=df[df[ele_name1] == i][ele_name3]
  plt.plot(index, sum, label=ele_name3)
  sum+=df[df[ele_name1] == i][ele_name4]
  plt.plot(index, sum, label=ele_name3+'+'+ele_name4)
  sum+=df[df[ele_name1] == i][ele_name5]
  plt.plot(index, sum, label=ele_name3+'+'+ele_name4+'+'+ele_name5)
  plt.xlabel("time")
  plt.ylabel("CPU Usage")
  plt.ylim(0,100)
  plt.title("{0}-{1}".format(os.path.basename(sys.argv[2]), i))
  plt.legend(bbox_to_anchor=(0, 1), loc='upper left', borderaxespad=1)
#  plt.savefig("{0}-{1}.png".format(sys.argv[2], i))
  plt.savefig(pp, format='pdf')
  plt.close('all')
pp.close()
