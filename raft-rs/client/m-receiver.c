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
  int (*send_req)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*send_ack)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*recv_req)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*recv_ack)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*free_req)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*free_ack)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*push_req)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*push_ack)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*helo_req)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*helo_ack)(int *epfd, int *fd, void *msg, struct cli_option opt);
  int (*timeout)(void *);
};

struct network_result client_register(int fd, int myid, int broker_id) {
  struct message msg;
  struct network_result net_res = net_send_msg(fd, msg_fill_hdr(&msg, MSG_HELO_REQ, myid, broker_id, 0));
  if (net_res.status != NETWORK_STATUS_SUCCESS) {
    return net_res;
  }
  net_res = net_recv_ack(fd, NULL, MSG_HELO_ACK, msg.hdr.id);
  return net_res;
}

// Reconnect and reregister myid
// Returns new fd
// Need close old fd by caller
int reregister_myid(int myid, struct cli_option opt, struct network_result net_res) {
  int fd;
  while (1) {
    sleep(1);
    // reconnect
    fd = net_reconnect(net_res, opt).data;
    if (fd < 0) {
      net_res.status = NETWORK_STATUS_ERR_CONNECT;
      continue;
    }
    // reregister
    net_res = client_register(fd, myid, MSG_ADDR_BROKER);
    if (net_res.status == NETWORK_STATUS_ERR_DO_REDIRECT) {
      close(fd);
      continue;
    } else if (net_res.status != NETWORK_STATUS_SUCCESS || net_res.data != MSG_TOTAL_LEN) {
      close(fd);
      net_res.status = NETWORK_STATUS_ERR_CONTENT;
      continue;
    }
    // success
    break;
  }
  return fd;
}

int update_epoll_fd(int *epfd, int old_fd, int new_fd)
{
  struct epoll_event ev;
  memset(&ev, 0, sizeof ev);

  ev.events = EPOLLIN;
  ev.data.fd = new_fd;

  if (epoll_ctl(*epfd, EPOLL_CTL_DEL, old_fd, NULL) != 0) {
    logger_error("epoll_ctl DEL failed\n");
    return -1;
  }

  if (epoll_ctl(*epfd, EPOLL_CTL_ADD, new_fd, &ev) != 0) {
    logger_error("epoll_ctl ADD failed\n");
    return -1;
  }
  return 0;
}

int client_event_loop(int *fd, struct cli_option opt, struct event_handlers *hd, void *context)
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
  ev.data.fd = *fd;

  if (epoll_ctl(epfd, EPOLL_CTL_ADD, *fd, &ev) != 0) {
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

    struct network_result net_res = net_recv_msg(*fd, &msg);
    while (net_res.status != NETWORK_STATUS_SUCCESS) {
      logger_warn("debug: not recv msg\n");
      int new_fd = reregister_myid(opt.myid, opt, net_res);
      update_epoll_fd(&epfd, *fd, new_fd);
      close(*fd);
      *fd = new_fd;

      // continue しないと 次の受信を感知できない？
      net_res = net_recv_msg(*fd, &msg);
    }

    n = net_res.data;
    if (n != MSG_TOTAL_LEN) {
      // connection closed
      printf("net_recv_msg failed: %d\n", n);
      break;
    }

    int (*handler_func)(int *epfd, int *fd, void *msg, struct cli_option opt) = NULL;
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
      if (handler_func(&epfd, fd, &msg, opt) != 0) {
        break;
      }
    } else {
    }

    if (count % 10000 == 0) logger_info("%d messages processed\n", count);
    count++;

  }

  // connection closed
  close(*fd);
  logger_info("connection closed from peer\n");
  return 0;
}

int message_arrived(int *epfd, int *fd, void *arg, struct cli_option opt)
{
  struct message *msg = (struct message *)arg;
  static int count = 0;

  // Something to do with msg here...

  int myid = msg->hdr.daddr;
  struct network_result net_res = net_send_ack(*fd, msg, myid);
  while (net_res.status != NETWORK_STATUS_SUCCESS)
  {
    logger_warn("debug: not send PushAck \n");
    int new_fd = reregister_myid(opt.myid, opt, net_res);
    update_epoll_fd(epfd, *fd, new_fd);
    close(*fd);
    *fd = new_fd;
    net_res = net_send_ack(*fd, msg, myid);
  }
  // logger_warn("debug: send PushAck \n");

  net_res = net_send_msg(*fd, msg_fill_hdr(msg, MSG_FREE_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
  while (net_res.status != NETWORK_STATUS_SUCCESS)
  {
    logger_warn("debug: not send FreeReq \n");
    int new_fd = reregister_myid(opt.myid, opt, net_res);
    update_epoll_fd(epfd, *fd, new_fd);
    close(*fd);
    *fd = new_fd;
    net_res = net_send_msg(*fd, msg_fill_hdr(msg, MSG_FREE_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
  }
  // logger_warn("debug: send FreeReq \n");

  net_res = net_recv_ack(*fd, NULL, MSG_FREE_ACK, msg->hdr.id);
  while (net_res.status != NETWORK_STATUS_SUCCESS)
  {
    if (net_res.status == NETWORK_STATUS_ERR_CONTENT) {
      logger_warn("debug: not recv FreeAck, retrying send FreeReq \n");
      net_res = net_send_msg(*fd, msg_fill_hdr(msg, MSG_FREE_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
      net_res = net_recv_ack(*fd, NULL, MSG_FREE_ACK, msg->hdr.id);
      continue;
    }
    logger_warn("debug: not recv FreeAck \n");
    int new_fd = reregister_myid(opt.myid, opt, net_res);
    update_epoll_fd(epfd, *fd, new_fd);
    close(*fd);
    *fd = new_fd;
    net_res = net_send_msg(*fd, msg_fill_hdr(msg, MSG_FREE_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
    net_res = net_recv_ack(*fd, NULL, MSG_FREE_ACK, msg->hdr.id);
  }
  // logger_warn("debug: recv FreeAck \n");

  if (count % 10000 == 0) logger_info("%d received, message: %s, id: %d\n", count, msg->payload, msg->hdr.id);
  count++;

  if (count >= max_count) {
    net_res = net_send_msg(*fd, msg_fill_hdr(msg, MSG_STAT_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
    while (net_res.status != NETWORK_STATUS_SUCCESS)
    {
      logger_warn("debug: not send StatReq \n");
      int new_fd = reregister_myid(opt.myid, opt, net_res);
      update_epoll_fd(epfd, *fd, new_fd);
      close(*fd);
      *fd = new_fd;
      net_res = net_send_msg(*fd, msg_fill_hdr(msg, MSG_STAT_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
    }

    net_res = net_recv_ack(*fd, NULL, MSG_STAT_RES, msg->hdr.id);
    while (net_res.status != NETWORK_STATUS_SUCCESS)
    {
      if (net_res.status == NETWORK_STATUS_ERR_CONTENT) {
        logger_warn("debug: not recv StatRes, retrying send StatReq \n");
        net_res = net_send_msg(*fd, msg_fill_hdr(msg, MSG_STAT_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
        net_res = net_recv_ack(*fd, NULL, MSG_STAT_RES, msg->hdr.id);
        continue;
      }
      logger_warn("debug: not recv StatRes \n");
      int new_fd = reregister_myid(opt.myid, opt, net_res);
      update_epoll_fd(epfd, *fd, new_fd);
      close(*fd);
      *fd = new_fd;
      net_res = net_recv_ack(*fd, NULL, MSG_STAT_RES, msg->hdr.id);
    }
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
  struct cli_option opt;
  init_cli_option(&opt); // clone default
  opt.myid = 0; // default my address

  if (cli_parse_option(argc, argv, &opt) < 0 || opt.receivers[0] != -1)
    usage_and_exit(-1);

  logger_set_current_level(opt.debug_level);
  logger_setup_log_file(opt.log_file, PROG_NAME);
  logger_setup_pid_file(opt.pid_file, PROG_NAME);
  logger_tinfo("* ", " %s started.\n", PROG_NAME);
  max_count = opt.msg_amount;

  struct network_result net_res = net_connect_recursive(opt.broker_host, opt.broker_port, (const char **)opt.broker_replicas, opt.broker_replicas_count);
  int fd = net_res.data;
  if (fd < 0) {
    exit(-1);
  }

  cli_fdump(logger_fp(LOG_DEBUG), &opt);

  net_res = client_register(fd, opt.myid, MSG_ADDR_BROKER);
  while (net_res.status != NETWORK_STATUS_SUCCESS) {
    logger_warn("failed to register myid %d. Now retry\n", opt.myid);
    close(fd);

    fd = net_reconnect(net_res, opt).data;
    net_res = client_register(fd, opt.myid, MSG_ADDR_BROKER);
  }

  int n = net_res.data;
  if (n < 0) {
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

  int result = client_event_loop(&fd, opt, &handler, NULL);
  close(fd);
  logger_warn("%s exiting.\n", PROG_NAME);
  return result;
}
