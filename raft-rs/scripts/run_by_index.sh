#!/usr/bin/env bash

usage() {
	echo "Usage: ./run_by_index.sh [-h] <position>"
	echo "This script is used to repeatedly run another script."
	echo
	echo "	-h        	display this help message"
	echo "	<position> 	the position of the script to run, starting from 1"
	echo
	echo "Example: ./run_by_index.sh 1"
	echo "You must write each script you want to run on a separate line in \$scripts"
	exit 1
}

if [ "$1" == "-h" ]; then
  usage
  exit 0
fi

if [ $# -ne 1 ]; then
  echo "error: Invalid number of arguments."
	echo "Please provide exactly 1 argument."
	echo
  usage
  exit 1
fi

scripts=(
	# example1.sh
	# example2.sh
	# etc...
	# write your scripts here, one per line
)

position_opt=$1
position=$((position_opt - 1))

script_to_run="${scripts[$position]}"
echo "${script_to_run}"
$script_to_run
