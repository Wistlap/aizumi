#pragma once

#include <stdio.h>
#include <stdint.h>

#define MSG_PAYLOAD_LEN 1024
#define MSG_HEADER_LEN  (sizeof(struct message_header))
#define MSG_TOTAL_LEN   (MSG_HEADER_LEN + MSG_PAYLOAD_LEN)

#define MSG_ADDR_BROKER 5000

struct message_header {
  uint32_t tot_len;  // total length including payload
  uint32_t msg_type; // MSG_SEND_REQ ..
  uint32_t saddr;    // source address
  uint32_t daddr;    // destination address
  uint32_t id;       // message Id
};

struct message {
  struct message_header hdr;
  char payload[MSG_PAYLOAD_LEN];
};

#define MSG_SEND_REQ  1 // sender -> broker (+payload)
#define MSG_SEND_ACK  2 // broker -> sender

#define MSG_RECV_REQ  3 // receiver -> broker
#define MSG_RECV_ACK  4 // broker -> receiver (+payload)

#define MSG_FREE_REQ  5 // receiver -> broker
#define MSG_FREE_ACK  6 // broker -> receiver

#define MSG_PUSH_REQ  7 // broker -> receiver (+payload)
#define MSG_PUSH_ACK  8 // receiver -> broker

#define MSG_HELO_REQ  9 // client -> broker
#define MSG_HELO_ACK 10 // broker -> client

#define MSG_STAT_REQ 11   // client -> broker (+stat_type)
#define MSG_STAT_RES 12   // broker -> client (+stat)

#define MSG_NACK 13 // generic negative ack. broker -> client (+new_leader_info)

const char* msg_type_to_string(int msg_type);
void msg_dump(struct message *msg);
void msg_fdump(FILE *fp, struct message *msg);
uint32_t message_get_id();

struct message *msg_fill(struct message *msg,
                         uint32_t msg_type,
                         uint32_t saddr,
                         uint32_t daddr,
                         uint32_t id,
                         void *payload,
                         int payload_len);

struct message *msg_fill_ack(struct message *msg,
                             struct message *orig_msg,
                             int myid);

struct message *msg_fill_hdr(struct message *msg,
                             uint32_t msg_type,
                             uint32_t saddr,
                             uint32_t daddr,
                             uint32_t id);

struct message *msg_fill_sprintf(struct message *msg,
                                 const char *fmt, ...);
