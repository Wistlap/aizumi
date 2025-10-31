#include <stdio.h>
#include <stdint.h>
#include <errno.h>
//#include <time.h>

#include "cli_parser.h"
#include "logger.h"
#include "network.h"
//#include "pthread.h"
#include "queue.h"
#include "timer.h"

#include "message_app.h"

#define PROG_NAME "m-amhs_com"
//#define MAX_EVENTS 1
//#define MAX_COUNT 200000

#define MY_ID_DEFAULT AMHS_COM_ID

#define TRANSFER_ASSIGN_RATE 2
#define TRANSFER_ACQUIRE_RATE 3
#define TRANSFER_DEPOSIT_RATE 5
#define TRANSFER_RATE_SUM (TRANSFER_ASSIGN_RATE+TRANSFER_ACQUIRE_RATE+TRANSFER_DEPOSIT_RATE)

struct event_rec
{
  uint64_t            send_tick_count;
  uint32_t*           send_pattern;
  uint32_t            cmd_id;
  uint32_t            source;
  uint32_t            dest;
  uint32_t            trans_ctl_id;
  struct event_rec*   next;
};

struct event_rec* EVENT_TABLE = NULL;

void insert_event(struct event_rec* event)
{
  //logger_info("insert command_id[%d]\n", event->cmd_id);
  // record empty
  if (EVENT_TABLE == NULL) {
    EVENT_TABLE = event;
    return;
  }

  struct event_rec* search_rec = EVENT_TABLE;

  // first record
  if (event->send_tick_count < search_rec->send_tick_count) {
    // change first record
    event->next = EVENT_TABLE;
    EVENT_TABLE = event;
    return;
  }

  while(search_rec->next != NULL) {
    if (event->send_tick_count < search_rec->next->send_tick_count) {
      // insert
      event->next = search_rec->next;
      search_rec->next = event;
      return;
    }
    search_rec = search_rec->next;
  }
  // search end
  search_rec->next = event;
}

void set_mic_transfer(const struct app_msg_mic_transfer_command* msg, uint32_t source_id)
{
  logger_tinfo("i ", "recv command_id[%d]source[%d]dest[%d]\n", msg->transfer_info.command_id, msg->transfer_info.source, msg->transfer_info.dest);
  struct event_rec *event[3];
  event[0] = malloc(sizeof(struct event_rec));
  event[1] = malloc(sizeof(struct event_rec));
  event[2] = malloc(sizeof(struct event_rec));
  uint64_t now = getTickCount();
  memset(event[0], 0x00, sizeof(struct event_rec));
  memset(event[1], 0x00, sizeof(struct event_rec));
  memset(event[2], 0x00, sizeof(struct event_rec));

  event[0]->send_tick_count = now + (msg->transfer_info.transfer_time / TRANSFER_RATE_SUM) * TRANSFER_ASSIGN_RATE;
  event[0]->send_pattern    = (uint32_t*)pattern1;
  event[0]->cmd_id          = msg->transfer_info.command_id;
  event[0]->source          = msg->transfer_info.source;
  event[0]->dest            = msg->transfer_info.dest;
  event[0]->trans_ctl_id    = source_id;
  insert_event(event[0]);

  event[1]->send_tick_count = now + (msg->transfer_info.transfer_time / TRANSFER_RATE_SUM) * (TRANSFER_ASSIGN_RATE + TRANSFER_ACQUIRE_RATE);
  event[1]->send_pattern    = (uint32_t*)pattern2;
  event[1]->cmd_id          = msg->transfer_info.command_id;
  event[1]->source          = msg->transfer_info.source;
  event[1]->dest            = msg->transfer_info.dest;
  event[1]->trans_ctl_id    = source_id;
  insert_event(event[1]);

  event[2]->send_tick_count = now + msg->transfer_info.transfer_time;
  event[2]->send_pattern    = (uint32_t*)pattern3;
  event[2]->cmd_id          = msg->transfer_info.command_id;
  event[2]->source          = msg->transfer_info.source;
  event[2]->dest            = msg->transfer_info.dest;
  event[2]->trans_ctl_id    = source_id;
  insert_event(event[2]);
}

void send_event(int fd, int myid, const struct event_rec* rec)
{
  struct app_msg_mic_transfer_event msg;

  for(size_t index = 0; rec->send_pattern[index] != END; ++index) {
    app_msg_fill_mic_transfer_event(&msg,
                                    rec->send_pattern[index],
                                    rec->cmd_id,
                                    rec->source,
                                    rec->dest);
    logger_tinfo("i ", "send command_id[%d] source[%d] dest[%d] event[%d]\n", msg.transfer_info.command_id,
                                                                  msg.transfer_info.source,
                                                                  msg.transfer_info.dest,
                                                                  msg.event_id);
    send_message_app(fd, myid, rec->trans_ctl_id, &msg, sizeof(msg));
  }
}

int on_recv_message(const uint32_t source_id, const uint32_t dest_id, const void* message, size_t size)
{
  const union app_msg* msg = message;
  //logger_info("recv message app_msg_type[%d]\n", msg->app_msg_type);
  switch (msg->app_msg_type)
  {
  case APP_MSG_MIC_TRANSFER_COMMAND:
    set_mic_transfer(&msg->mic_transfer, source_id);
    break;
  case 999:
    return -1;
  default:
    logger_error("app_msg_type:[%d]error\n", msg->app_msg_type);
    break;
  }
  return 0;
}

int on_timeup(int fd, int myid)
{
  uint64_t now = getTickCount();

  //if (EVENT_TABLE == NULL) {
  //  logger_info("on_timeup now[%ld] no data\n", now);
  //} else {
  //  logger_info("on_timeup now[%ld] timeup[%ld]\n", now, EVENT_TABLE->send_tick_count);
  //}

  while(EVENT_TABLE != NULL) {
    if(EVENT_TABLE->send_tick_count > now) {
      break;
    }
    //logger_info("now[%ld] send_tick_count[%ld]\n", now, EVENT_TABLE->send_tick_count);
    // send event
    send_event(fd, myid, EVENT_TABLE);
    // delete record
    struct event_rec* next = EVENT_TABLE->next;
    free(EVENT_TABLE);
    EVENT_TABLE = next;
  }
  return 0;
}

int message_arrived(int fd, void *arg)
{
  struct message *msg = (struct message *)arg;
  static int count = 1;

  // Something to do with msg here...

  int myid = msg->hdr.daddr;
  net_send_ack(fd, msg, myid);

  int ret = on_recv_message(msg->hdr.saddr, myid, msg->payload, MSG_PAYLOAD_LEN);

  net_send_msg(fd, msg_fill_hdr(msg, MSG_FREE_REQ, myid, MSG_ADDR_BROKER, msg->hdr.id));
  //net_recv_ack(fd, NULL, MSG_FREE_ACK, msg->hdr.id);  // when received SEND_REQ, error occurred.

  if (count % 10000 == 0) logger_info("%d received, message: %d, id: %d\n", count, *(uint32_t *)msg->payload, msg->hdr.id);
  count++;

  return ret;
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
  opt.myid = MY_ID_DEFAULT; // default my address

  if (cli_parse_option(argc, argv, &opt) < 0 || opt.receivers[0] != -1)
    usage_and_exit(-1);

  logger_set_current_level(opt.debug_level);
  logger_setup_log_file(opt.log_file, PROG_NAME);
  logger_setup_pid_file(opt.pid_file, PROG_NAME);
  logger_tinfo("* ", " %s started.\n", PROG_NAME);

  struct network_result net_res = net_connect(opt.broker_host, opt.broker_port);
  int fd = net_res.data;

  if (fd < 0) {
    logger_error("failed to connect %s:%s\n",
                 opt.broker_host, opt.broker_port);
    exit(-1);
  }

  cli_fdump(logger_fp(LOG_DEBUG), &opt);

  if (client_register2(fd, opt.myid, MSG_ADDR_BROKER) < 0) {
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
    .timeout  = on_timeup,
  };

  int result = client_event_loop2(fd, opt.myid, &handler, NULL, 300);
  close(fd);
  logger_tinfo("* ", " %s stopped (%d).\n", PROG_NAME, result);
  return result;
}
