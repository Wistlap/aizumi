#include <stdio.h>
#include <stdint.h>
#include <errno.h>
#include <time.h>

#include "cli_parser.h"
#include "logger.h"
#include "network.h"
//#include "pthread.h"
#include "queue.h"
#include "timer.h"

#include "message_app.h"

#define PROG_NAME "m-trans_ctl"
//#define MAX_EVENTS 1
//#define MAX_COUNT 200000

#define MY_ID_DEFAULT TRANS_CTL_ID

// global variable
char file_name[128];
uint64_t start_tick_count;

struct mac_transfer_data
{
  uint32_t trans_id;
  uint32_t source;
  uint32_t dest;
  uint32_t running_count;
  struct mic_transfer_info micro[3];
  uint64_t start[3];
  //uint64_t comp[3];
  uint64_t counter[15];
  struct mac_transfer_data* next;
};

static struct mac_transfer_data* TRANSFER_TABLE;

void add_transfer_data(struct mac_transfer_data* data)
{
  if(TRANSFER_TABLE == NULL){
    TRANSFER_TABLE = data;
    return;
  }
  struct mac_transfer_data** search_end = &(TRANSFER_TABLE->next);
  while(*search_end != NULL) {
    search_end = &((*search_end)->next);
  }
  *search_end = data;
}

struct mac_transfer_data* search_record(const uint32_t command_id,
                                        const uint32_t source,
                                        const uint32_t dest)
{
  struct mac_transfer_data* search = TRANSFER_TABLE;
  while(search != NULL) {
    //logger_info("running_count[%d]\n", search->running_count);
    if (search->running_count == 0) {
      // invalid record
      continue;
    } else {
      //logger_info("cmd[%d]src[%d]dst[%d]==cmd[%d]src[%d]dst[%d]\n", search->micro[search->running_count - 1].command_id,
      //                                                         search->micro[search->running_count - 1].source,
      //                                                         search->micro[search->running_count - 1].dest,
      //                                                         command_id, source, dest);
      if(search->micro[search->running_count - 1].command_id == command_id &&
         search->micro[search->running_count - 1].source == source &&
         search->micro[search->running_count - 1].dest == dest) {
        return search;
      }
    }
    search = search->next;
  }
  return NULL;
}

void delete_record(const struct mac_transfer_data* rec)
{
  if(TRANSFER_TABLE == NULL){
    logger_error("no data\n");
    return;
  }
  if(TRANSFER_TABLE == rec){
    struct mac_transfer_data* temp = TRANSFER_TABLE;
    TRANSFER_TABLE = TRANSFER_TABLE->next;
    free(temp);
    return;
  }

  struct mac_transfer_data* search = TRANSFER_TABLE;
  while(search->next != NULL) {
    if(search->next == rec){
      struct mac_transfer_data* temp = search->next;
      search->next = search->next->next;
      free(temp);
      return;
    }
    search = search->next;
  }
  logger_error("not found data\n");
  return;
}

void set_mac_transfer(const struct app_msg_mac_transfer_command* message, struct mac_transfer_data** data, const uint64_t counter)
{
  *data = malloc(sizeof(struct mac_transfer_data));
  memset(*data, 0x00, sizeof(struct mac_transfer_data));
  struct mac_transfer_data* transfer = *data;
  transfer->trans_id = message->transfer_id;
  transfer->source = message->source;
  transfer->dest = message->dest;
  transfer->running_count = 1;
  transfer->micro[0].command_id = message->transfer_info[0].command_id;
  transfer->micro[0].source = message->transfer_info[0].source;
  transfer->micro[0].dest = message->transfer_info[0].dest;
  transfer->micro[0].transfer_time = message->transfer_info[0].transfer_time;
  transfer->micro[1].command_id = message->transfer_info[1].command_id;
  transfer->micro[1].source = message->transfer_info[1].source;
  transfer->micro[1].dest = message->transfer_info[1].dest;
  transfer->micro[1].transfer_time = message->transfer_info[1].transfer_time;
  transfer->micro[2].command_id = message->transfer_info[2].command_id;
  transfer->micro[2].source = message->transfer_info[2].source;
  transfer->micro[2].dest = message->transfer_info[2].dest;
  transfer->micro[2].transfer_time = message->transfer_info[2].transfer_time;
  transfer->start[0] = getTickCount() - start_tick_count;;
  transfer->next = NULL;
  transfer->counter[0] = counter;
  add_transfer_data(transfer);
}

void recv_event(int fd, const uint32_t source_id, const uint32_t dest_id, const struct app_msg_mic_transfer_event* message, const uint64_t counter)
{
  logger_info("recv command_id[%d]source[%d]dest[%d]event[%d]\n", message->transfer_info.command_id,
                                                             message->transfer_info.source,
                                                             message->transfer_info.dest,
                                                             message->event_id);
  // search record
  struct mac_transfer_data* rec = search_record(message->transfer_info.command_id,
                                                message->transfer_info.source,
                                                message->transfer_info.dest);
  if (rec == NULL) {
    logger_error("Search Error\n");
    return;
  }

  rec->counter[message->event_id] = rec->counter[message->event_id] + counter;

  if (message->event_id != EVENT_TRANSFER_COMPLETED) {
    return;
  }

  // incriment micro transfer
  if (1 <= rec->running_count && rec->running_count <= 2) {
    ++(rec->running_count);
    // send transfer data to amhs_com
    struct app_msg_mic_transfer_command msg;  // trans_ctl -> amhs_com
    memset(&msg, 0x00, sizeof(msg));
    msg.app_msg_type   = APP_MSG_MIC_TRANSFER_COMMAND;
    msg.transfer_info.command_id    = rec->micro[rec->running_count - 1].command_id;
    msg.transfer_info.source        = rec->micro[rec->running_count - 1].source;
    msg.transfer_info.dest          = rec->micro[rec->running_count - 1].dest;
    msg.transfer_info.transfer_time = rec->micro[rec->running_count - 1].transfer_time;
    rec->start[rec->running_count - 1] = getTickCount() - start_tick_count;

    logger_info("send cmd[%d]src[%d]dst[%d]time[%d]\n", msg.transfer_info.command_id,
                                                        msg.transfer_info.source,
                                                        msg.transfer_info.dest,
                                                        msg.transfer_info.transfer_time);
    send_message_app(fd, dest_id, AMHS_COM_ID, (void*)&msg, sizeof(msg));
  } else if(rec->running_count == 3) {
    // macro transfer completed
    // send transfer data to amhs_com
    struct app_msg_mac_transfer_comp msg;  // trans_ctl -> host_com
    app_msg_fill_mac_transfer_comp(&msg, rec->trans_id);

    FILE *fp;
    fp = fopen(file_name, "a");
    uint64_t comp = getTickCount() - start_tick_count;
    fprintf(fp, "%d,%ld,%ld,%ld,%ld,%ld,%ld,%ld,%ld,%lu,%lu,%lu,%lu,%lu,%lu,%lu,%lu,%lu,%lu,%lu,%lu,%lu,%lu,%lu\n",
            rec->trans_id,
            rec->start[0], rec->start[1] - rec->start[0],
            rec->start[1], rec->start[2] - rec->start[1],
            rec->start[2], comp - rec->start[2],
            comp, comp - rec->start[0],
            rec->counter[0], rec->counter[1], rec->counter[2], rec->counter[3], rec->counter[4],
            rec->counter[5], rec->counter[6], rec->counter[7], rec->counter[8], rec->counter[9],
            rec->counter[10], rec->counter[11], rec->counter[12], rec->counter[13], rec->counter[14]);
    fclose(fp);

    logger_info("send to host_com trans_id[%d]\n", msg.transfer_id);
    send_message_app(fd, dest_id, HOST_COM_ID, (void*)&msg, sizeof(msg));

    // delete record
    delete_record(rec);
  }
}

int on_recv_message(int fd, const uint32_t source_id, const uint32_t dest_id, const void* message, size_t size)
{
  const union app_msg* msg = message;
  uint64_t counter;
  //const char *dump = message;
  //logger_info("recv %02X %02X %02X %02X %02X %02X %02X %02X %02X %02X\n", dump[0], dump[1], dump[2], dump[3], dump[4], dump[5], dump[6], dump[7], dump[8], dump[9]);

  switch (msg->app_msg_type)
  {
  case APP_MSG_MAC_TRANSFER_COMMAND:
    busy_loop(90, &counter);
    logger_info("recv trans_id[%d]source[%d]dest[%d]size[%ld]\n", msg->mac_transfer.transfer_id, msg->mac_transfer.source, msg->mac_transfer.dest, size);

    struct mac_transfer_data* data = NULL;
    set_mac_transfer(&msg->mac_transfer, &data, counter);

    // send transfer data to amhs_com
    struct app_msg_mic_transfer_command sendMsg;  // trans_ctl -> amhs_com
    app_msg_fill_mic_transfer_command(&sendMsg,
                                      data->micro[0].command_id,
                                      data->micro[0].source,
                                      data->micro[0].dest,
                                      data->micro[0].transfer_time);
    logger_info("send command_id[%d]source[%d]dest[%d]\n", sendMsg.transfer_info.command_id, sendMsg.transfer_info.source, sendMsg.transfer_info.dest);
    send_message_app(fd, dest_id, AMHS_COM_ID, (void*)&sendMsg, sizeof(sendMsg));
    break;
  case APP_MSG_MIC_TRANSFER_EVENT:
    busy_loop(15, &counter);
    recv_event(fd, source_id, dest_id, &msg->mic_transfer_event, counter);
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
  // no action
  return 0;
}

int message_arrived(int fd, void *arg)
{
  struct message *msg = (struct message *)arg;
  static int count = 1;

  // Something to do with msg here...

  int myid = msg->hdr.daddr;
  net_send_ack(fd, msg, myid);

  int ret = on_recv_message(fd, msg->hdr.saddr, myid, msg->payload, MSG_PAYLOAD_LEN);

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

  // make file name
  time_t now = time(NULL);
  struct tm tm;
  localtime_r(&now, &tm);
  sprintf(file_name, "log/m-trans_log%04d%02d%02d%02d%02d%02d_%04d.csv",
                      tm.tm_year + 1900,
                      tm.tm_mon + 1,
                      tm.tm_mday,
                      tm.tm_hour,
                      tm.tm_min,
                      tm.tm_sec,
                      opt.myid);
  FILE *fp;
  fp = fopen(file_name, "a");
  fprintf(fp, "trans_id,start1(ms),time1(ms),start2(ms),time2(ms),start3(ms),time3(ms),comp3(ms),total(ms),cnt0,cnt1,cnt2,cnt3,cnt4,cnt5,cnt6,cnt7,cnt8,cnt9,cnt10,cnt11,cnt12,cnt13,cnt14\n");
  fclose(fp);

  start_tick_count = getTickCount();  // initial value

  int result = client_event_loop2(fd, opt.myid, &handler, NULL, 1000);
  close(fd);
  logger_tinfo("* ", " %s stopped (%d).\n", PROG_NAME, result);
  return result;
}
