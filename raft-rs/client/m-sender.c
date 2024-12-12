#include <stdio.h>
#include <stdlib.h>

#include "network.h"
#include "cli_parser.h"
#include "logger.h"
#include "queue.h"
#include "statistics.h"

#define PROG_NAME "m-sender"

int send_logsignals(int fd, int myid, int *receiver_ids, int npackets, void *payload) {
  struct message smsg;
  struct message ack;
  union status *stat = malloc(sizeof(union status));
  int stat_type = OUTPUT_LOG_AND_EXIT;
  memcpy(smsg.payload, &stat_type, sizeof(int));
  for (int i=0; i<npackets; i++) {
    msg_fill_hdr(&smsg, MSG_STAT_REQ, myid, 5000, 0);
    net_send_msg(fd, &smsg);
    net_recv_msg(fd, &ack);
    memcpy(stat, ack.payload, sizeof(union status));
    logger_info("%d done\n", i);
  }
  return 0;
}

int register_myid(int fd, int myid) {
  struct message msg;
  net_send_msg(fd, msg_fill_hdr(&msg, MSG_HELO_REQ, myid, MSG_ADDR_BROKER, 0));
  return net_recv_ack(fd, NULL, MSG_HELO_ACK, msg.hdr.id);
}

void send_stat_req(int fd, int myid, int stat_type) {
  struct message smsg;
  struct message ack;
  union status *stat = malloc(sizeof(union status));
  memcpy(smsg.payload, &stat_type, sizeof(int));
  msg_fill_hdr(&smsg, MSG_STAT_REQ, myid, MSG_ADDR_BROKER, 0);
  net_send_msg(fd, &smsg);
  net_recv_msg(fd, &ack);
  memcpy(stat, ack.payload, sizeof(union status));
  logger_info("sum_queues: %d, max_messages: %d, sum_messages: %d\n", stat->queue_stat.sum_queues, stat->queue_stat.max_messages, stat->queue_stat.sum_messages);
}

int send_messages(int fd, int myid, int *receiver_ids, int npackets, void *payload) {
  struct message smsg;
  int *orig_receiver_ids = receiver_ids;
  for (int i = 0; i < npackets; i++) {
    receiver_ids = orig_receiver_ids;
    while (*receiver_ids >= 0) {
      msg_fill_hdr(&smsg, MSG_SEND_REQ, myid, *receiver_ids, 0);
      msg_fill_sprintf(&smsg, "Hello from sender %d", myid);
      net_send_msg(fd, &smsg);
      net_recv_ack(fd, NULL, MSG_SEND_ACK, smsg.hdr.id);
    receiver_ids++;
    }
  }
  return 0;
}

void usage_and_exit(int status)
{
  fprintf(stderr, "Usage: %s [-d LOG_LEVEL] [-b BROKER:PORT] [-c MSGS]\
 [-l LOG-FILE] [-p PID-FILE] [-u MYID]\
 RECEIVER...\n", PROG_NAME);
  exit(status);
}

int main(int argc, char *argv[])
{
  struct cli_option opt = CLI_OPTION_DEFAULT; // clone default
  opt.myid = 10000; // default my address

  if (cli_parse_option(argc, argv, &opt) < 0 || opt.receivers[0] == -1)
    usage_and_exit(-1);

  logger_set_current_level(opt.debug_level);
  logger_setup_log_file(opt.log_file, PROG_NAME);
  logger_setup_pid_file(opt.pid_file, PROG_NAME);
  logger_tinfo("* ", " %s started.\n", PROG_NAME);

  int fd = net_connect(opt.broker_host, opt.broker_port);

  if (fd < 0) {
    logger_error("failed to connect %s:%s\n",
                 opt.broker_host, opt.broker_port);
    exit(-1);
  }

  int n = register_myid(fd, opt.myid);

  if (n != MSG_TOTAL_LEN) {
    logger_error("could not registered.\n");
    close(fd);
    exit(-1);
  }

  // send messages for each receiver up to amount, with empty payload
  cli_fdump(logger_fp(LOG_DEBUG), &opt);

  char payload[MSG_PAYLOAD_LEN] = "Hello";
  // send message or send logsignal
  if (opt.logsignal == 1) {
    send_logsignals(fd, opt.myid, &opt.receivers[0], opt.msg_amount, payload);
  } else {
    send_messages(fd, opt.myid, &opt.receivers[0], opt.msg_amount, payload);
    send_stat_req(fd, opt.myid, QUEUE_STAT);
  }
  close(fd);

  return 0;
}
