#!/bin/bash

sed 1d $1 | sed '/idle$/d' | sed '1s/^$/time cpu usr nice sys iowait irq soft steal guest gnice idle/' | sed '/^$/d' | sed '/^平均値/d' > ./tmp.tsv

OUTPUT_DIR="$(dirname $1)"
OUTPUT_FILE="$(basename $1 .log)"
cp ./tmp.tsv $OUTPUT_DIR/$OUTPUT_FILE.tsv
rm -f ./tmp.tsv
