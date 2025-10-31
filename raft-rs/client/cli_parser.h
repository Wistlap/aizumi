#pragma once

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <stdint.h>

struct cli_option {
  int debug_level;          // debug level (0 is not in debug-mode)
  char broker_host[128];    // "localhost"
  char broker_port[128];    // "3000" or "http"
  int broker_timeout;       // epoll timeout
  int broker_threads;       // broker_threads (default: 10)
  int msg_amount;           // How many packets do you blast?
  char log_file[4096];      // "m-sender01.log"
  char pid_file[4096];      // "m-sender01.pid"
  int myid;                 // 10000
  int trans_ctl_process;    // number of trans_ctl processes
  uint32_t mph;             // moves per hour
  uint32_t trans_count;     // number of transfer commands
  int receivers[1000];      // 1, 2, 3, 4...
  int logsignal;            // 0 or 1
  char **broker_replicas;          // broker candidates "127.0.0.1:3000,127.0.0.1:3000,..."
  int broker_replicas_count;        // for count broker candidates. It is not command line argument
};

#define OPT_DEFAULT_MYID_SENDER 10000
#define OPT_DEFAULT_MYID_RECEIVER 0
#define OPT_DEFAULT_MYID_BROKER 50000

#define OPT_SET_STRING(var_name, src_string)                    \
  {strncpy(var_name, src_string, sizeof(var_name) - 1);}

#define OPT_SET_NUMBER(var_name, src_string)    \
  {var_name = atoi(src_string);}

#define OPT_IS_EMPTY(var_name)                  \
  (strcmp((var_name), "") == 0)

// Supporsed to be filled in cli_parser.c
extern struct cli_option CLI_OPTION_DEFAULT;

void cli_dump(struct cli_option *opt);
void cli_fdump(FILE *fp, struct cli_option *opt);
int cli_parse_option(int argc, char * const argv[], struct cli_option *parsed_opt);
void init_cli_option(struct cli_option *opt);
