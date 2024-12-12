#!/usr/bin/env python

import pypdf
import sys

file = sys.argv[1]
pdf = pypdf.PdfReader(file)
print('used dirs :', pdf.metadata['/Dirs'])
