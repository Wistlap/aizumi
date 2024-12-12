#include <stdio.h>
#include <stdint.h>
#include <errno.h>
#include <time.h>

#include "cli_parser.h"
#include "logger.h"
#include "network.h"
#include "queue.h"
#include "timer.h"

#include "message_app.h"

#define PROG_NAME "m-host_com"

#define MY_ID_DEFAULT HOST_COM_ID

// global variable
char file_name[128];
uint64_t start_tick_count;
int trans_ctl_num;  // number of m=trans_ctl processes
uint32_t trans_id;  // transfer ID
uint64_t timeup;
uint32_t mph;         // moves per hour
uint32_t max_trans_count; // number of transfer commands

// host transfer command
struct mac_transfer
{
  uint32_t source;
  uint32_t dest;
};

// host transfer command lists
const struct mac_transfer MACRO_TRANSFER_TABLE[] = {
  {100, 400},
  {101, 401},
  {102, 402},
  {103, 403},
  {104, 404},
  {105, 405},
  {106, 406},
  {107, 407},
  {108, 408},
  {109, 409},
};
#define TRANSFER_MAX sizeof(MACRO_TRANSFER_TABLE) / sizeof(struct mac_transfer)

// micro transfer info
//struct mic_transfer
//{
//  uint32_t source;
//  uint32_t dest;
//  uint32_t time;
//};

// macro -> micro exchange struct
struct route
{
  uint32_t source;
  uint32_t dest;
  struct mic_transfer_info micro[3];
};

// macro -> micro exchange table
struct route ROUTE_TABLE[] = {
  {100, 400, {{0, 100, 200, 40000},{0, 200, 300, 40000},{0, 300, 400, 40000}}},
  {101, 401, {{0, 101, 201, 40000},{0, 201, 301, 40000},{0, 301, 401, 40000}}},
  {102, 402, {{0, 102, 202, 40000},{0, 202, 302, 40000},{0, 302, 402, 40000}}},
  {103, 403, {{0, 103, 203, 40000},{0, 203, 303, 40000},{0, 303, 403, 40000}}},
  {104, 404, {{0, 104, 204, 40000},{0, 204, 304, 40000},{0, 304, 404, 40000}}},
  {105, 405, {{0, 105, 205, 40000},{0, 205, 305, 40000},{0, 305, 405, 40000}}},
  {106, 406, {{0, 106, 206, 40000},{0, 206, 306, 40000},{0, 306, 406, 40000}}},
  {107, 407, {{0, 107, 207, 40000},{0, 207, 307, 40000},{0, 307, 407, 40000}}},
  {108, 408, {{0, 108, 208, 40000},{0, 208, 308, 40000},{0, 308, 408, 40000}}},
  {109, 409, {{0, 109, 209, 40000},{0, 209, 309, 40000},{0, 309, 409, 40000}}},
  {END, END, {{END,END, END, END},{END,END, END, END},{END,END, END, END}}},
};
#define ROUTE_MAX sizeof(ROUTE_TABLE) / sizeof(struct route)

const struct route* route_search(const struct mac_transfer* mac_trans)
{
  struct route* loop = ROUTE_TABLE;

  while(loop->source != END) {
    if (loop->source == mac_trans->source && loop->dest == mac_trans->dest) {
      return loop;
    }
    ++loop;
  }
  return NULL;  // route not found
}

// waiting list for transfer completed
struct mac_transfer_data
{
  uint32_t trans_id;
  uint32_t source;
  uint32_t dest;
  uint64_t start;
  //uint64_t end;
  struct mac_transfer_data* next;
};

static struct mac_transfer_data* TRANSFER_TABLE;
static size_t transfer_count;

void add_transfer_data(struct mac_transfer_data* data)
{
  if(TRANSFER_TABLE == NULL) {
    TRANSFER_TABLE = data;
    transfer_count = 1;
    return;
  }
  struct mac_transfer_data** search_end = &(TRANSFER_TABLE->next);
  while(*search_end != NULL) {
    search_end = &((*search_end)->next);
  }
  *search_end = data;
  ++transfer_count;
}

struct mac_transfer_data* search_record(const uint32_t trans_id)
{
  struct mac_transfer_data* search = TRANSFER_TABLE;
  while(search != NULL) {
    //logger_info("running_count[%d]\n", search->running_count);
    if(search->trans_id == trans_id) {
      return search;
    }
    search = search->next;
  }
  return NULL;
}

void delete_record(const struct mac_transfer_data* rec)
{
  if(TRANSFER_TABLE == NULL){
    logger_error("no data\n");
    transfer_count = 0;
    return;
  }
  if(TRANSFER_TABLE == rec){
    struct mac_transfer_data* temp = TRANSFER_TABLE;
    TRANSFER_TABLE = TRANSFER_TABLE->next;
    free(temp);
    --transfer_count;
    return;
  }

  struct mac_transfer_data* search = TRANSFER_TABLE;
  while(search->next != NULL) {
    if(search->next == rec){
      struct mac_transfer_data* temp = search->next;
      search->next = search->next->next;
      free(temp);
      --transfer_count;
      return;
    }
    search = search->next;
  }
  logger_error("not found data\n");
  return;
}

void set_mac_transfer(const struct app_msg_mac_transfer_command* message)
{
  struct mac_transfer_data* data = malloc(sizeof(struct mac_transfer_data));
  data->trans_id = message->transfer_id;
  data->source = message->source;
  data->dest = message->dest;
  data->start = getTickCount() - start_tick_count;
  data->next = NULL;
  add_transfer_data(data);
}

void recv_transfer_comp(const struct app_msg_mac_transfer_comp* message, const uint32_t source_id)
{
  struct mac_transfer_data* data = search_record(message->transfer_id);
  if (data == NULL) {
    logger_error("rearch error trans_id:[%d]\n", message->transfer_id);
    return;
  }

  FILE *fp;
  fp = fopen(file_name, "a");
  uint64_t comp = getTickCount() - start_tick_count;
  fprintf(fp, "%d,%ld,%ld,%ld,%ld,%d\n", message->transfer_id, data->start, comp, comp - data->start, transfer_count, source_id);
  fclose(fp);

  delete_record(data);

  logger_info("Transfer Completed:[%d]\n", message->transfer_id);
}

int on_recv_message(const uint32_t source_id, const uint32_t dest_id, const void* message, size_t size)
{
  const union app_msg* msg = message;

  switch (msg->app_msg_type)
  {
  case APP_MSG_MAC_TRANSFER_COMP:
    //busy_loop(50);
    recv_transfer_comp(&msg->mac_transfer_comp, source_id);
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
  static size_t transfer_record_num = 0;
  if (timeup == 0) {
    timeup = getTickCount();  // initial value
  }
  static uint32_t trans_ctl_offset = 0;

  // debug
  if (trans_id > max_trans_count * trans_ctl_num) {
    if (transfer_count == 0) {
      return -1;  // exit
    }
    return 0;
  }

  if (getTickCount() < timeup) {
    return 0;
  }
  timeup += 3600*1000/mph/trans_ctl_num; // mph transfers per trans_ctl process

  // send transfer command
  const struct mac_transfer* mac_trans = &MACRO_TRANSFER_TABLE[transfer_record_num];

  // search transfer route
  const struct route* trans_route = route_search(mac_trans);
  if (trans_route == NULL) {
    return 0;
  }

  struct app_msg_mac_transfer_command msg;
  app_msg_fill_mac_transfer_command(&msg,
                                    trans_id,
                                    trans_route->source,
                                    trans_route->dest,
                                    trans_route->micro);

  set_mac_transfer(&msg);

  logger_info("send[%d]source[%d]dest[%d]size[%ld]\n", msg.transfer_id, msg.source, msg.dest, sizeof(msg));
  send_message_app(fd, myid, TRANS_CTL_ID + trans_ctl_offset, (void*)&msg, sizeof(msg));

  ++trans_id;
  ++transfer_record_num;
  if (transfer_record_num >= TRANSFER_MAX) {
    transfer_record_num = 0;
  }

  ++trans_ctl_offset;
  if (trans_ctl_offset >= trans_ctl_num) {
    trans_ctl_offset = 0;
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
  [-l LOG-FILE] [-p PID-FILE] [-u MYID] [-x TRANS_PCOUNT] [-m MPH/TRANSFERS]\
  \n", PROG_NAME);
  exit(status);
}

int main(int argc, char *argv[])
{
  struct cli_option opt = CLI_OPTION_DEFAULT; // clone default
  opt.myid = MY_ID_DEFAULT; // default my address

  if (cli_parse_option(argc, argv, &opt) < 0 || opt.receivers[0] != -1)
    usage_and_exit(-1);

  mph = opt.mph;
  max_trans_count = opt.trans_count;

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

  int result = false;
  for(trans_ctl_num = opt.trans_ctl_process; trans_ctl_num <= opt.trans_ctl_process; ++trans_ctl_num) {
    // make file name
    time_t now = time(NULL);
    struct tm tm;
    localtime_r(&now, &tm);
    sprintf(file_name, "log/m-host_log%04d%02d%02d%02d%02d%02dP%02d.csv",
                       tm.tm_year + 1900,
                       tm.tm_mon + 1,
                       tm.tm_mday,
                       tm.tm_hour,
                       tm.tm_min,
                       tm.tm_sec,
                       trans_ctl_num);
    FILE *fp;
    fp = fopen(file_name, "a");
    fprintf(fp, "trans_id, start(ms),comp(ms),time(ms),trans_count,trans_ctl_ID\n");
    fclose(fp);

    start_tick_count = getTickCount();  // initial value
    trans_id = 1;
    timeup = 0;

    result = client_event_loop2(fd, opt.myid, &handler, NULL, 50);
  }

  struct app_msg_mac_transfer_command msg;
  memset(&msg, 0x00, sizeof(msg));
  msg.app_msg_type = 999;
  send_message_app(fd, opt.myid, HOST_COM_ID, (void*)&msg, sizeof(msg));
  send_message_app(fd, opt.myid, AMHS_COM_ID, (void*)&msg, sizeof(msg));
  for(int i = 0; i < opt.trans_ctl_process; ++i) {
    send_message_app(fd, opt.myid, TRANS_CTL_ID + i, (void*)&msg, sizeof(msg));
  }

  close(fd);
  logger_tinfo("* ", " %s stopped (%d).\n", PROG_NAME, result);
  return result;
}
