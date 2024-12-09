#include <stdio.h>
#include <sys/epoll.h>

#include "cli_parser.h"
#include "logger.h"
#include "message.h"
#include "network.h"
#include "pthread.h"
#include "queue.h"
#include "timer.h"

#define PROG_NAME "m-receiver"
#define MAX_EVENTS 1
#define MAX_COUNT 200000

int max_count = 0;

struct th_arg {
  int fd;
  int id;
};

struct event_handlers {
  int (*send_req)(int fd, void *msg);
  int (*send_ack)(int fd, void *msg);
  int (*recv_req)(int fd, void *msg);
  int (*recv_ack)(int fd, void *msg);
  int (*free_req)(int fd, void *msg);
  int (*free_ack)(int fd, void *msg);
  int (*push_req)(int fd, void *msg);
  int (*push_ack)(int fd, void *msg);
  int (*helo_req)(int fd, void *msg);
  int (*helo_ack)(int fd, void *msg);
  int (*timeout)(void *);
};

int client_register(int fd, int myid, int broker_id) {
  struct message msg;
  net_send_msg(fd, msg_fill_hdr(&msg, MSG_HELO_REQ, myid, broker_id, 0));
  return net_recv_ack(fd, NULL, MSG_HELO_ACK, msg.hdr.id);
}

int client_event_loop(int fd, int myid, struct event_handlers *hd, void *context)
{
  int n, nfd, count = 0;
  struct message msg;
  struct epoll_event ev;
  struct epoll_event events[MAX_EVENTS];
  // struct timer_storage *tscs = timer_create_storage(MAX_COUNT);

  int epfd = epoll_create(MAX_EVENTS);

  if (epfd < 0) {
    logger_error("epoll_create");
    return -1;
  }
  memset(&ev, 0, sizeof ev);

  ev.events = EPOLLIN;
  ev.data.fd = fd;

  if (epoll_ctl(epfd, EPOLL_CTL_ADD, fd, &ev) != 0) {
    logger_error("epoll_ctl");
  }

  while (1) {
    nfd = epoll_wait(epfd, events, MAX_EVENTS, 1000);

    // Timeout from epoll_wait
    if (nfd == 0) {
      // printf("timeout call handler_func\n");
      // hd->timeout(NULL);
      continue;
    }

    if ((n = net_recv_msg(fd, &msg)) != MSG_TOTAL_LEN) {
      // connection closed
      // printf("net_recv_msg failed: %d\n", n);
      break;
    }

    int (*handler_func)(int fd, void *msg) = NULL;
    switch (msg.hdr.msg_type) {
    case MSG_SEND_REQ: handler_func = hd->send_req; break;
    case MSG_SEND_ACK: handler_func = hd->send_ack; break;
    case MSG_RECV_REQ: handler_func = hd->recv_req; break;
    case MSG_RECV_ACK: handler_func = hd->recv_ack; break;
    case MSG_FREE_REQ: handler_func = hd->free_req; break;
    case MSG_FREE_ACK: handler_func = hd->free_ack; break;
    case MSG_PUSH_REQ: handler_func = hd->push_req; break;
    case MSG_PUSH_ACK: handler_func = hd->push_ack; break;
    case MSG_HELO_REQ: handler_func = hd->helo_req; break;
    case MSG_HELO_ACK: handler_func = hd->helo_ack; break;
    default:
      logger_warn("invalid message type %s (%d) ignored.",
                  msg_type_to_string(msg.hdr.msg_type),
                  msg.hdr.msg_type
                  );
      break;
    }

    if (handler_func) {
      if (handler_func(fd, &msg) != 0) {
        break;
      }
    } else {
    }

    if (count % 10000 == 0) logger_info("%d messages processed\n", count);
    count++;

  }

  // connection closed
  close(fd);
  logger_info("connection closed from peer\n");
  return 0;
}

int message_arrived(int fd, void *arg)
{
  struct message *msg = (struct message *)arg;
  static int count = 0;

  // Something to do with msg here...

  int myid = msg->hdr.daddr;
  net_send_ack(fd, msg, myid);

  net_send_msg(fd, msg_fill_hdr(msg, MSG_FREE_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
  net_recv_ack(fd, NULL, MSG_FREE_ACK, msg->hdr.id);

  if (count % 10000 == 0) logger_info("%d received, message: %s, id: %d\n", count, msg->payload, msg->hdr.id);
  count++;

  if (count >= max_count) {
    net_send_msg(fd, msg_fill_hdr(msg, MSG_STAT_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
    net_recv_ack(fd, NULL, MSG_STAT_RES, msg->hdr.id);
    return -1; // exit event loop
  } else {
    return 0;
  }
}

void usage_and_exit(int status)
{
  fprintf(stderr, "Usage: %s [-d LOG_LEVEL] [-b BROKER:PORT] [-c MSGS]\
 [-l LOG-FILE] [-p PID-FILE] [-u MYID]\
\n", PROG_NAME);
  exit(status);
}

int main(int argc, char *argv[])
{
  struct cli_option opt = CLI_OPTION_DEFAULT; // clone default
  opt.myid = 0; // default my address

  if (cli_parse_option(argc, argv, &opt) < 0 || opt.receivers[0] != -1)
    usage_and_exit(-1);

  logger_set_current_level(opt.debug_level);
  logger_setup_log_file(opt.log_file, PROG_NAME);
  logger_setup_pid_file(opt.pid_file, PROG_NAME);
  logger_tinfo("* ", " %s started.\n", PROG_NAME);
  max_count = opt.msg_amount;

  int fd = net_connect(opt.broker_host, opt.broker_port);

  if (fd < 0) {
    logger_error("failed to connect %s:%s\n",
                 opt.broker_host, opt.broker_port);
    exit(-1);
  }

  cli_fdump(logger_fp(LOG_DEBUG), &opt);

  if (client_register(fd, opt.myid, MSG_ADDR_BROKER) < 0) {
    logger_error("could not registered.\n");
    close(fd);
    exit(-1);
  }

  struct event_handlers handler = {
    .send_req = NULL,
    .send_ack = NULL,
    .recv_req = NULL,
    .recv_ack = NULL,
    .free_req = NULL,
    .free_ack = NULL,
    .push_req = message_arrived,
    .push_ack = NULL,
    .helo_req = NULL,
    .helo_ack = NULL,
    .timeout  = NULL,
  };

  int result = client_event_loop(fd, opt.myid, &handler, NULL);
  close(fd);
  return result;
}
