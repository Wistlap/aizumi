//#include <mqueue.h>
//#include <stdint.h>
#include <sys/epoll.h>

#include "message_app.h"
#include "network.h"
#include "message.h"
#include "logger.h"

#define MAX_EVENTS 1
#define MAX_COUNT 200000

void app_msg_fill_mac_transfer_command(struct app_msg_mac_transfer_command* msg,
                                       const uint32_t transfer_id,
                                       const uint32_t source,
                                       const uint32_t dest,
                                       const struct mic_transfer_info* transfer_info)
{
  memset(msg, 0x00, sizeof(struct app_msg_mac_transfer_command));
  msg->app_msg_type   = APP_MSG_MAC_TRANSFER_COMMAND;
  msg->transfer_id  = transfer_id;
  msg->source       = source;
  msg->dest         = dest;
  msg->transfer_info[0].command_id    = transfer_id;
  msg->transfer_info[0].source        = transfer_info[0].source;
  msg->transfer_info[0].dest          = transfer_info[0].dest;
  msg->transfer_info[0].transfer_time = transfer_info[0].transfer_time;
  msg->transfer_info[1].command_id    = transfer_id;
  msg->transfer_info[1].source        = transfer_info[1].source;
  msg->transfer_info[1].dest          = transfer_info[1].dest;
  msg->transfer_info[1].transfer_time = transfer_info[1].transfer_time;
  msg->transfer_info[2].command_id    = transfer_id;
  msg->transfer_info[2].source        = transfer_info[2].source;
  msg->transfer_info[2].dest          = transfer_info[2].dest;
  msg->transfer_info[2].transfer_time = transfer_info[2].transfer_time;
}

void app_msg_fill_mic_transfer_command(struct app_msg_mic_transfer_command *msg,
                                       const uint32_t command_id,
                                       const uint32_t source,
                                       const uint32_t dest,
                                       const uint32_t transfer_time)
{
  memset(msg, 0x00, sizeof(struct app_msg_mic_transfer_command));
  msg->app_msg_type   = APP_MSG_MIC_TRANSFER_COMMAND;
  msg->transfer_info.command_id    = command_id;
  msg->transfer_info.source        = source;
  msg->transfer_info.dest          = dest;
  msg->transfer_info.transfer_time = transfer_time;
}

void app_msg_fill_mic_transfer_event(struct app_msg_mic_transfer_event *msg,
                                     const uint32_t event_id,
                                     const uint32_t command_id,
                                     const uint32_t source,
                                     const uint32_t dest)
{
  memset(msg, 0x00, sizeof(struct app_msg_mic_transfer_event));
  msg->app_msg_type = APP_MSG_MIC_TRANSFER_EVENT;
  msg->event_id = event_id;
  msg->transfer_info.command_id = command_id;
  msg->transfer_info.source = source;
  msg->transfer_info.dest = dest;
}

void app_msg_fill_mac_transfer_comp(struct app_msg_mac_transfer_comp *msg,
                                    const uint32_t transfer_id)
{
  memset(msg, 0x00, sizeof(struct app_msg_mac_transfer_comp));
  msg->app_msg_type   = APP_MSG_MAC_TRANSFER_COMP;
  msg->transfer_id    = transfer_id;
}

#if 1
int send_message_app(int fd, int myid, int receiver_id, void *payload, size_t size) {
  struct message smsg;
  msg_fill(&smsg, MSG_SEND_REQ, myid, receiver_id, 0, payload, size);
  net_send_msg(fd, &smsg);
  //net_recv_ack(fd, NULL, MSG_SEND_ACK, smsg.hdr.id);  // when received SEND_REQ, error occurred.
  return 0;
}
#else
int send_message_app(int fd, int myid, int receiver_id, void *payload, size_t size) {
  int ret;
  struct mq_attr attr;
  mqd_t mqddes;

  char *dest;
  switch(receiver_id)
  {
  case HOST_COM_ID:
    dest = "/m-host_com";
    break;
  case TRANS_CTL_ID:
    dest = "/m-trans_ctl";
    break;
  case AMHS_COM_ID:
    dest = "/m-amhs_com";
    break;
  default:
    return -1;
  }

  // name is same as recv process.
  mqddes = mq_open(dest, O_WRONLY);
  if (mqddes < 0) {
      logger_error("mq_open failed, mqddes:[%d], errno:[%d]\n", mqddes, errno);
      return mqddes;
  }

  // set queue attribute
  mq_getattr(mqddes ,&attr);
  //logger_info("attr mq_maxmsg:[%ld], mq_msgsize:[%ld]\n", attr.mq_maxmsg, attr.mq_msgsize);

  // send message priority is 1
  ret = mq_send(mqddes, (const char*)msg, size, 1);
  if (ret < 0) {
      logger_error("mq_send failed, mqddes:[%d], errno:[%d]\n", mqddes, errno);
      return ret;
  }

  mq_close(mqddes);
  return 0;
}
#endif

int client_register2(int fd, int myid, int broker_id) {
  struct message msg;
  net_send_msg(fd, msg_fill_hdr(&msg, MSG_HELO_REQ, myid, broker_id, 0));
  struct network_result net_res = net_recv_ack(fd, NULL, MSG_HELO_ACK, msg.hdr.id);
  return net_res.data;
}

#if 1
int client_event_loop2(const int fd, const int myid, const struct event_handlers *hd, const void *context, const uint32_t interval)
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
    nfd = epoll_wait(epfd, events, MAX_EVENTS, interval);

    // Timeout from epoll_wait
    if (nfd == 0) {
      // logger_info("timeout call handler_func\n");
      if (hd->timeout(fd, myid) != 0) {
        break;
      }
      continue;
    }

    struct network_result net_res = net_recv_msg(fd, &msg);
    if ((n = net_res.data) != MSG_TOTAL_LEN) {
      // connection closed
      logger_error("net_recv_msg failed: %d\n", n);
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
      logger_warn("invalid message type %s (%d) ignored.\n",
                  msg_type_to_string(msg.hdr.msg_type),
                  msg.hdr.msg_type
                  );
      break;
    }

    if (handler_func) {
      int ret = handler_func(fd, &msg);
      if (ret != 0) {
        logger_warn("handler_func return (%d)\n", ret);
        break;
      }
    }
    if (hd->timeout(fd, myid) != 0) {
      break;
    }

    if (count % 10000 == 0) logger_info("%d messages processed\n", count);
    count++;
  }

  // connection closed
  //close(fd);
  //logger_info("connection closed from peer\n");
  return 0;
}
#else
int client_event_loop2(const int fd, const int myid, const struct event_handlers *hd, const void *context, const uint32_t interval)
{
  // temporary communication start
  ssize_t rcv_size;
  struct mq_attr attr;
  mqd_t mqddes;
  struct timespec timeout;
  unsigned int priority;
  char msg[1024];
  const char *que_name = "/m-host_com";

  mq_unlink(que_name);

  // set queue attribute
  attr.mq_flags = O_NONBLOCK;
  attr.mq_maxmsg = 200;
  attr.mq_msgsize = 1024;

  mqddes = mq_open(que_name, O_RDONLY | O_CREAT, S_IRWXU, &attr);
  if (mqddes < 0) {
      logger_error("mq_open failed mqddes:[%d], errno:[%d]\n", mqddes, errno);
      return -1;
  }
  logger_info("recv loop start...\n");
  while(1) {
    clock_gettime(CLOCK_REALTIME, &timeout);

    if ((timeout.tv_nsec + interval * 1000000) >= 1000 * 1000000) {
      timeout.tv_sec += 1;
      timeout.tv_nsec = timeout.tv_nsec + interval * 1000000 - 1000 * 1000000;
    } else {
      timeout.tv_nsec = timeout.tv_nsec + interval * 1000000;
    }

    // recv message with timeout
    rcv_size = mq_timedreceive(mqddes, msg, 1024, &priority, &timeout);
    if (rcv_size < 0) {
        if (errno == 110) {
            // timeout
            //logger_info("call on_timeup()\n");
            on_timeup();
            continue;
        }

        logger_error("mq_timedreceive failed rcv_size:[%ld], errno:[%d]\n", rcv_size, errno);

        mq_close(mqddes);
        mq_unlink(que_name);
        return rcv_size;
    }
    on_recv_message(0, 0, msg, rcv_size);
    on_timeup();
  }
  // temporary communication end
}
#endif
