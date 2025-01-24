#!/usr/bin/env bash

# use this script in scripts directory

#re make log directory
echo "re make log directory"
rm -f ../log
mkdir ../log
echo
sleep 1

# sender-receiver: 1-1,1-10,1-20,10-1,10-10,10-20,20-1,20-10.20-20, raft nodes: 5
killall rast-rs-broker
./autotest_all.sh -b ../broker/target/release/raft-rs-broker -r 1,10,20 -s 1,10,20 -c 100000 -d 3 -l "../log/n_clients_5_nodes/raft-rs-broker-%s-%r-%t-%c-%g-$(date +%Y%m%d-%H%M%S).log" -- -t 1 -g 5
echo
sleep 1

# sender-receiver: 1-1, raft nodes: 1-10
killall rast-rs-broker
./autotest_all.sh -b ../broker/target/release/raft-rs-broker -r 1 -s 1 -c 100000 -d 3 -l "../log/1_clients_n_nodes/raft-rs-broker-%s-%r-%t-%c-%g-$(date +%Y%m%d-%H%M%S).log" -- -t 1 -g 1-10
echo
sleep 1

# sender-receiver: 10-10, raft nodes: 1-10
killall rast-rs-broker
./autotest_all.sh -b ../broker/target/release/raft-rs-broker -r 10 -s 10 -c 100000 -d 3 -l "../log/10_clients_n_nodes/raft-rs-broker-%s-%r-%t-%c-%g-$(date +%Y%m%d-%H%M%S).log" -- -t 1 -g 1-10
echo
sleep 1

# make stat files
for log in ../log/*
do
  echo $log
  if [ ! -f "$log/*.stat" ]; then
    # echo ./csv2stat_all.sh $log
    ./csv2stat_all.sh $log
  fi
done
echo
sleep 1

# make graph
echo "make graph from stat files in n_clients_5_nodes directory"
./stat2graph.py ../log/n_clients_5_nodes -tb
for file in *.pdf; do
  if [ "$file" ]; then
    mv "$file" "n_clients_5_nodes.pdf"
    break
  fi
done
echo
sleep 1

echo "make graph from stat files in 1_clients_n_nodes and 10_clients_n_nodes directories"
./stat2graph_2.py ../log/1_clients_n_nodes ../log/10_clients_n_nodes -tb
for file in *.pdf; do
  if [ "$file" != "n_clients_5_nodes.pdf" ]; then
    mv "$file" "1_and_10_clients_n_nodes.pdf"
    break
  fi
done
echo
sleep 1

mv n_clients_5_nodes.pdf 1_and_10_clients_n_nodes.pdf ../log

echo "done"
