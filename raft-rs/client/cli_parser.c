#include "cli_parser.h"

struct cli_option CLI_OPTION_DEFAULT = {
  .debug_level = 2,
  .broker_host = "localhost",
  .broker_port = "5555",
  .broker_timeout = 1,
  .broker_threads = 10,
  .msg_amount = 1000,
  .log_file = "",
  .pid_file = "",
  .myid = 0,
  .trans_ctl_process = 1,
  .mph = 5000,
  .trans_count = 6000,
  .receivers = {-1},
  .logsignal = 0,
};

// "100"     -> head = 100, tail = 100
// "100-200" -> head = 100, tail = 200
static int cli_parse_range(char *range_string, int *head, int *tail) {
  char *p = range_string;
  *head = 0, *tail = 0;

  while ('0' <= *p && *p <= '9') {
    *head = *head * 10 + (*p++ - '0');
  }
  switch (*p) {
  case '\0':
    *tail = *head;
    break;
  case '-':
    *tail = atoi(p + 1);
    break;
  default:
    return -1;
  }
  if (*head > *tail)
    return -1;
  return 0;
}

void cli_dump(struct cli_option *opt)
{
  cli_fdump(stdout, opt);
}

void cli_fdump(FILE *fp, struct cli_option *opt)
{
  if (!fp) return;

  fprintf(fp, "sizeof struct cli_option: %ld\n", sizeof(struct cli_option));
  fprintf(fp, "debug_level: %d\n", opt->debug_level);
  fprintf(fp, "broker_host: %s\n", opt->broker_host);
  fprintf(fp, "broker_port: %s\n", opt->broker_port);
  fprintf(fp, "broker_timeout: %d\n", opt->broker_timeout);
  fprintf(fp, "broker_threads: %d\n", opt->broker_threads);
  fprintf(fp, "msg_amount: %d\n", opt->msg_amount);
  fprintf(fp, "log-file: %s\n", OPT_IS_EMPTY(opt->log_file) ? "NONE" : opt->log_file);
  fprintf(fp, "pid-file: %s\n", OPT_IS_EMPTY(opt->pid_file) ? "NONE" : opt->pid_file);
  fprintf(fp, "myid: %d\n", opt->myid);
  fprintf(fp, "receiver-id:");
  fprintf(fp, "logsignal:");

  for (int i = 0; i < 1000; i++) {
    if (opt->receivers[i] < 0) break;
    fprintf(fp, " %d", opt->receivers[i]);
  }
  fprintf(fp, "\n");
}

int cli_parse_option(int argc, char * const argv[], struct cli_option *parsed_opt)
{
  int opt;
  char buff[64];

  for (int i = 0; i < 1000; i++) {
    parsed_opt->receivers[i] = -1;
  }

  while ((opt = getopt(argc, argv, "b:c:d:l:m:n:p:t:u:x:s:")) != -1) {
    switch (opt) {
    case 'b':
      OPT_SET_STRING(parsed_opt->broker_host, optarg);
      strtok(parsed_opt->broker_host, ":");
      char *port = strtok(NULL, ":");
      if (!port) return -1;
      OPT_SET_STRING(parsed_opt->broker_port, port);
      break;
    case 'c':
      OPT_SET_NUMBER(parsed_opt->msg_amount, optarg);
      break;
    case 'd':
      OPT_SET_NUMBER(parsed_opt->debug_level, optarg);
      break;
    case 'l':
      OPT_SET_STRING(parsed_opt->log_file, optarg);
      break;
    case 'm':
      strcpy(buff, optarg);
      strtok(buff, "/");
      OPT_SET_NUMBER(parsed_opt->mph, buff);
      char *trans = strtok(NULL, "/");
      if (!trans) return -1;
      OPT_SET_NUMBER(parsed_opt->trans_count, trans);
    case 'n':
      OPT_SET_NUMBER(parsed_opt->broker_threads, optarg);
      break;
    case 'p':
      OPT_SET_STRING(parsed_opt->pid_file, optarg);
      break;
    case 't':
      OPT_SET_NUMBER(parsed_opt->broker_timeout, optarg);
      break;
    case 'u':
      OPT_SET_NUMBER(parsed_opt->myid, optarg);
      break;
    case 'x':
      OPT_SET_NUMBER(parsed_opt->trans_ctl_process, optarg);
      break;
    case 's':
      OPT_SET_NUMBER(parsed_opt->logsignal, optarg);
      break;
    default:
      return -1;
    }
  }

  int head, tail, index = 0;

  while (optind < argc) {
    if (cli_parse_range(argv[optind], &head, &tail) < 0) {
      return -1;
    }
    for (int recid = head; recid <= tail; recid++) {
      parsed_opt->receivers[index++] = recid;
    }
    optind++;
  }
  return 0;
}
