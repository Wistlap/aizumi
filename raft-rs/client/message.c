#include <ctype.h>
#include <stdarg.h>
#include <string.h>
#include <stdbool.h>

#include "message.h"
#include "pthread.h"

////////////////////////////////////////////////////////////////
/// private functions

static uint32_t current_message_id = 1; // XXX: should be thread-safe
pthread_mutex_t id_mp = PTHREAD_MUTEX_INITIALIZER;

static int get_corresponding_ack_type(int msg_type)
{
  // XXX quick hack!
  return msg_type + 1;
}

////////////////////////////////////////////////////////////////
/// public functions

#define MSG_NUM_TO_STRING(MSG_NUM) #MSG_NUM
#define MSG_SYM_TO_STRING(MSG_SYM) #MSG_SYM "(" MSG_NUM_TO_STRING(MSG_SYM) ")"

const char* msg_type_to_string(int msg_type)
{
  switch(msg_type) {
  case MSG_SEND_REQ: return MSG_SYM_TO_STRING(MSG_SEND_REQ);
  case MSG_SEND_ACK: return MSG_SYM_TO_STRING(MSG_SEND_ACK);
  case MSG_RECV_REQ: return MSG_SYM_TO_STRING(MSG_RECV_REQ);
  case MSG_RECV_ACK: return MSG_SYM_TO_STRING(MSG_RECV_ACK);
  case MSG_FREE_REQ: return MSG_SYM_TO_STRING(MSG_FREE_REQ);
  case MSG_FREE_ACK: return MSG_SYM_TO_STRING(MSG_FREE_ACK);
  case MSG_PUSH_REQ: return MSG_SYM_TO_STRING(MSG_PUSH_REQ);
  case MSG_PUSH_ACK: return MSG_SYM_TO_STRING(MSG_PUSH_ACK);
  case MSG_HELO_REQ: return MSG_SYM_TO_STRING(MSG_HELO_REQ);
  case MSG_HELO_ACK: return MSG_SYM_TO_STRING(MSG_HELO_ACK);
  case MSG_STAT_REQ: return MSG_SYM_TO_STRING(MSG_STAT_REQ);
  case MSG_STAT_RES: return MSG_SYM_TO_STRING(MSG_STAT_RES);
  case MSG_NACK:     return MSG_SYM_TO_STRING(MSG_NACK);
  default: return "MSG_UNKNOWN";
  }
}

bool msg_is_ack(struct message *msg) {
  switch(msg->hdr.msg_type) {
  case MSG_SEND_ACK:
  case MSG_RECV_ACK:
  case MSG_FREE_ACK:
  case MSG_PUSH_ACK:
  case MSG_HELO_ACK:
    return true;
  default:
    return false;
  }
}

bool msg_is_nack(struct message *msg) {
  return msg->hdr.msg_type == MSG_NACK;
}

void msg_dump(struct message *msg)
{
  msg_fdump(stdout, msg);
}

void msg_fdump(FILE *fp, struct message *msg)
{
  if (!fp) return;

  fprintf(fp, "mesage(id=%u tot_len=%u msg_type=%s saddr=%u daddr=%u printable_payload=",
          msg->hdr.id,
          msg->hdr.tot_len,
          msg_type_to_string(msg->hdr.msg_type),
          msg->hdr.saddr,
          msg->hdr.daddr);
  if (msg_is_ack(msg)) {
    fprintf(fp, "%d", *((int*)msg->payload));
  } else {
    for (int i = 0; i < MSG_PAYLOAD_LEN; i++) {
      if (!isprint(msg->payload[i])) break;
      fprintf(fp, "%c", msg->payload[i]);
    }
  }
  fprintf(fp, ")\n");
  fflush(fp);
}

//
// Fill message header.
//
// If id is zero, filled by some uniq value.
//
struct message *msg_fill_hdr(struct message *msg,
                             uint32_t msg_type,
                             uint32_t saddr,
                             uint32_t daddr,
                             uint32_t id) {

  msg->hdr.tot_len = MSG_TOTAL_LEN; // XXX currently, fixed-length is assumed.
  msg->hdr.msg_type = msg_type;
  msg->hdr.saddr = saddr;
  msg->hdr.daddr = daddr;
  if (id == 0) {
    pthread_mutex_lock(&id_mp);
    id = current_message_id++;
    pthread_mutex_unlock(&id_mp);
  }
  msg->hdr.id = id; // filled if id is zero.

  return msg;
}

uint32_t message_get_id() {
  pthread_mutex_lock(&id_mp);
  int id = current_message_id++;
  pthread_mutex_unlock(&id_mp);
  return id;
}

//
// fill msg.payload in the sprintf manner.
//
struct message *msg_fill_sprintf(struct message *msg, const char *fmt, ...)
{
  va_list ap;
  va_start(ap, fmt);
  vsnprintf(msg->payload, MSG_PAYLOAD_LEN, fmt, ap);
  va_end(ap);

  return msg;
}

struct message *msg_fill_ack(struct message *msg,
                             struct message *orig_msg,
                             int myid) {

  int msg_type = get_corresponding_ack_type(orig_msg->hdr.msg_type);
  return msg_fill(msg, msg_type, myid, orig_msg->hdr.saddr, 0, &orig_msg->hdr.id, sizeof(orig_msg->hdr.id));
}

struct message *msg_fill(struct message *msg, uint32_t msg_type,
                         uint32_t saddr, uint32_t daddr, uint32_t id,
                         void *payload, int payload_len)
{
  msg_fill_hdr(msg, msg_type, saddr, daddr, id);

  if (payload && (msg_type != MSG_STAT_RES)) {
    memcpy(msg->payload, payload,
           payload_len > MSG_PAYLOAD_LEN ? MSG_PAYLOAD_LEN : payload_len);
  }
  //printf("fill %02X %02X %02X %02X %02X %02X %02X %02X %02X %02X\n", msg->payload[0], msg->payload[1], msg->payload[2], msg->payload[3], msg->payload[4], msg->payload[5], msg->payload[6], msg->payload[7], msg->payload[8], msg->payload[9]);
  return msg;
}
