#!/usr/bin/env bash

TOPDIR=$(dirname "$0")

if [ $# == 0 ]; then
    NETENV="local"
else
    NETENV=$1
fi

PID_FILE="$TOPDIR/broker.pid"
date=$(date +"%Y%m%d-%H%M%S")

function is_broker_working() {
  test $(ps h -L $(cat $PID_FILE) | wc -l) != 1
}

function is_receiver_working() {
  test $(ps | grep m-receiver | wc -l) != 0
}

function sequence() {
  local seq="$1"

  for e in $(sed 's/,/ /g' <<<"$seq"); do
    case "$e" in
      [0-9]*-[0-9]*)
        head=$(cut -f1 -d- <<<"$e")
        tail=$(cut -f2 -d- <<<"$e")
        seq -s ' ' "$head" "$tail"
        ;;
      *)
        echo "$e"
        ;;
    esac
  done | tr '\n' ' '
  echo "" # add LF
}

function expand_optargs() {
  local opt_key="$1"
  local opt_seq="$2"

  for v in $(sequence "$opt_seq"); do
    echo "$opt_key $v"
  done
}

function expand_broker_opt() {
    if [ $# -gt 0 ]; then
        local BO=$(expand_optargs "$1" "$2")
        shift 2
        local boseq=$(expand_broker_opt "$@")
        cartesian_product "$BO" "$boseq"
    fi
}

function cartesian_product() {
  local seq1="$1"
  local seq2="$2"
  shift 2

  if [ $# -gt 0 ]; then
    local ptmp=$(cartesian_product "$seq1" "$seq2")
    cartesian_product "$ptmp" "$@"
    return
  fi

  local LF=$'\n'
  while read -d "$LF" e1; do
    while read -d "$LF" e2; do
      echo "$e1 $e2"
    done <<<"$seq2"
  done <<<"$seq1"
}

function get_optarg() {
  local optname="$1"
  shift

  while [ $# -gt 0 ]
  do
    case "$1" in
      -$optname)
        echo "$2"
        return
        ;;
    esac
    shift
  done
}

function replace_optargs() {
  local before="$@"
  local after=

  while [ $# -gt 0 ]
  do
    # find %X style keys in "$1": "a-%b-%c-d" → "b\nc\n"
    local optnames=$(echo "$1" | sed 's/[^%]*%\(.\)/\1\n/g' | sed '$d')

    # replace %Xs in current_arg
    local current_arg="$1"
    for name in $optnames
    do
      optarg=$(get_optarg "$name" $before | sed "s![./]!!g")
      current_arg=$(echo "$current_arg" | sed "s!%$name!$optarg!g")
    done
    after="$after $current_arg"
    shift
  done
  echo "$after"
}

# Parse option

usage() {
    echo "Usage: autotest_all.sh -b BROKER -r SEQ -s SEQ [-c N] [-d LEVEL] [-l LOG-DIR] [--replica SEQ] [--runtime RUNTIME] [-- -BROKER-OPT]"
    echo "  -b BROKER     Broker path"
    echo "  -c N          Number of messages (default:100000)"
    echo "  -d LEVEL      Set LOG level (default:2)"
    echo "  -l LOG-FILE   Set broker's log file (default: stdout) and Use mpstat and make graph"
    echo "  -r SEQ        Range of number of receiver processes"
    echo "  -s SEQ        Range of number of sender processes"
    echo " --replica      Range of number of replica processes"
    echo " --runtime      Runtime name"
    echo "  -BROKER-OPT   Set the option that depend on the broker used"
}

# set default values
OPT_N_MESSAGES=100000
OPT_LOG_LEVEL=2
OPT_LOG_FILE="stdout"
OPT_SEQ_REPLICAS=0
OPT_RUNTIME=""

args=$(getopt -o b:c:d:hl:r:s: --long replica:,runtime:: -- "$@") || exit 1
eval "set -- $args"

while [ $# -gt 0 ]; do
  case "$1" in
    -h)
        OPT_ERROR=1;
        break ;;
    -b)
        OPT_BROKER=$2;
        shift 2      ;;
    -c)
        OPT_N_MESSAGES=$2;
        shift 2 ;;
    -d)
        OPT_LOG_LEVEL=$2;
        shift 2 ;;
    -l)
        OPT_LOG_FILE=$2;
        shift 2        ;;
    -r)
        OPT_SEQ_RECEIVERS=$2;
        shift 2 ;;
    -s)
        OPT_SEQ_SENDERS=$2;
        shift 2   ;;
    '--replica')
        OPT_SEQ_REPLICAS=$2;
        shift 2  ;;
    '--runtime')
        OPT_RUNTIME=$2;
        shift 2 ;;
    '--')
        shift;
        break ;;
  esac
done

# unknown option check
if [ "$OPT_ERROR" = 1 ]; then
  usage
  exit 1
fi

# Set the options in variables
BROKER="$OPT_BROKER"
BROKER_NAME="$(basename $BROKER)"
MESSAGE_COUNT="$OPT_N_MESSAGES"
LOG_LEVEL="$OPT_LOG_LEVEL"
LOG_FILE="$OPT_LOG_FILE"
LOG_DIR=$(dirname $LOG_FILE)
SEQ_RECEIVERS="$OPT_SEQ_RECEIVERS"
SEQ_SENDERS="$OPT_SEQ_SENDERS"
SEQ_REPLICAS="$OPT_SEQ_REPLICAS"
RUNTIME="$OPT_RUNTIME"
BROKER_OPT="$@"

if [ "$LOG_FILE" != "stdout" ]; then
    mkdir -p "$LOG_DIR"
fi

S=$(expand_optargs "-s" $SEQ_SENDERS)
R=$(expand_optargs "-r" $SEQ_RECEIVERS)
REP=$(expand_optargs "--replica" $SEQ_REPLICAS)
BO=$(expand_broker_opt $BROKER_OPT)

if [ "$LOG_FILE" != "stdout" ]; then
  cartesian_product "$R" "$S" "$REP" "--" "$BO" | while read -r cond_opt; do
    TEST="$TOPDIR/autotest.sh --runtime=$RUNTIME -l $LOG_FILE -b $BROKER -c $MESSAGE_COUNT -d $LOG_LEVEL $cond_opt"
    COMMAND=$(replace_optargs $TEST)
    echo $COMMAND
    $COMMAND
  done
else
  cartesian_product "$R" "$S" "$REP" "--" "$BO" | while read -r cond_opt; do
    TEST="$TOPDIR/autotest.sh --runtime=$RUNTIME -b $BROKER -c $MESSAGE_COUNT -d $LOG_LEVEL $cond_opt"
    echo $TEST
    $TEST
  done
fi
