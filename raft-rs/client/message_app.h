#pragma once
#include <stdint.h>

#include "message.h"

#define HOST_COM_ID 1001
#define TRANS_CTL_ID 2001
#define AMHS_COM_ID 3001

// Message define wetween application process
#define APP_MSG_MAC_TRANSFER_COMMAND   1 // host_comm -> trans_ctl
#define APP_MSG_MIC_TRANSFER_COMMAND   2 // trans_ctl -> amhs_com
#define APP_MSG_MIC_TRANSFER_EVENT     3 // amhs_com -> trans_ctl
#define APP_MSG_MAC_TRANSFER_COMP      4 // trans_ctl -> host_comm

// define of event from amhs_com to trans_ctl
#define EVENT_TRANSFER_INITIATED         1
#define EVENT_VEHICLE_ASSIGNED           2
#define EVENT_VEHICLE_ARRIVED            3
#define EVENT_TRANSFERRING               4
#define EVENT_VEHICLE_ACQUIRE_STARTED    5
#define EVENT_CARRIER_INSTALLED          6
#define EVENT_VEHICLE_ACQUIRE_COMPLETED  7
#define EVENT_VEHICLE_DEPARTED           8
#define EVENT_VEHICLE_ARRIVED2           9
#define EVENT_VEHICLE_DEPOSIT_STARTED    10
#define EVENT_CARRIER_REMOVED            11
#define EVENT_VEHICLE_DEPOSIT_COMPLETED  12
#define EVENT_VEHICLE_UNASSIGNED         13
#define EVENT_TRANSFER_COMPLETED         14
#define END                              0

// event report pattern
static const uint32_t pattern1[] = {EVENT_TRANSFER_INITIATED,
                                    EVENT_VEHICLE_ASSIGNED,
                                    END};  // start micro transfer
static const uint32_t pattern2[] = {EVENT_VEHICLE_ARRIVED,
                                    EVENT_TRANSFERRING,
                                    EVENT_VEHICLE_ACQUIRE_STARTED,
                                    EVENT_CARRIER_INSTALLED,
                                    EVENT_VEHICLE_ACQUIRE_COMPLETED,
                                    EVENT_VEHICLE_DEPARTED,
                                    END};  // loading start~complete
static const uint32_t pattern3[] = {EVENT_VEHICLE_ARRIVED2,
                                    EVENT_VEHICLE_DEPOSIT_STARTED,
                                    EVENT_CARRIER_REMOVED,
                                    EVENT_VEHICLE_DEPOSIT_COMPLETED,
                                    EVENT_VEHICLE_UNASSIGNED,
                                    EVENT_TRANSFER_COMPLETED,
                                    END}; // unloading start~complete

// micro transfer info
struct mic_transfer_info
{
  uint32_t command_id;
  uint32_t source;
  uint32_t dest;
  uint32_t transfer_time;
};

// application message struct
#define APP_MSG_PADDING_LEN   MSG_PAYLOAD_LEN-4*5-sizeof(struct mic_transfer_info)*3

struct app_msg_mac_transfer_command // host_comm -> trans_ctl
{
  uint32_t app_msg_type; // application message type
  uint32_t transfer_id;
  uint32_t source;
  uint32_t dest;
  uint32_t padding5;
  struct mic_transfer_info transfer_info[3];
  char padding7[APP_MSG_PADDING_LEN];
};
void app_msg_fill_mac_transfer_command(struct app_msg_mac_transfer_command* msg,
                                       const uint32_t transfer_id,
                                       const uint32_t source,
                                       const uint32_t dest,
                                       const struct mic_transfer_info* transfer_info);

struct app_msg_mic_transfer_command // trans_ctl -> amhs_com
{
  uint32_t app_msg_type; // application message type
  uint32_t padding2;
  uint32_t padding3;
  uint32_t padding4;
  uint32_t padding5;
  struct mic_transfer_info transfer_info;
  struct mic_transfer_info padding6[2];
  char padding7[APP_MSG_PADDING_LEN];
};
void app_msg_fill_mic_transfer_command(struct app_msg_mic_transfer_command* msg,
                                       const uint32_t command_id,
                                       const uint32_t source,
                                       const uint32_t dest,
                                       const uint32_t transfer_time);

struct app_msg_mic_transfer_event // amhs_com -> trans_ctl
{
  uint32_t app_msg_type; // application message type
  uint32_t padding2;
  uint32_t padding3;
  uint32_t padding4;
  uint32_t event_id;
  struct mic_transfer_info transfer_info;
  struct mic_transfer_info padding6[2];
  char padding7[APP_MSG_PADDING_LEN];
};
void app_msg_fill_mic_transfer_event(struct app_msg_mic_transfer_event* msg,
                                     const uint32_t event_id,
                                     const uint32_t command_id,
                                     const uint32_t source,
                                     const uint32_t dest);

struct app_msg_mac_transfer_comp  // trans_ctl -> host_comm
{
  uint32_t app_msg_type; // application message type
  uint32_t transfer_id;
  uint32_t padding3;
  uint32_t padding4;
  uint32_t padding5;
  struct mic_transfer_info padding6[3];
  char padding7[APP_MSG_PADDING_LEN];
};
void app_msg_fill_mac_transfer_comp(struct app_msg_mac_transfer_comp* msg,
                                    const uint32_t transfer_id);

// all application message
union app_msg
{
  uint32_t app_msg_type; // application message type
  struct app_msg_mac_transfer_command mac_transfer;       // host_comm -> trans_ctl
  struct app_msg_mic_transfer_command mic_transfer;       // trans_ctl -> amhs_com
  struct app_msg_mic_transfer_event   mic_transfer_event; // amhs_com -> trans_ctl
  struct app_msg_mac_transfer_comp    mac_transfer_comp;  // trans_ctl -> host_comm
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
  int (*timeout)(int fd, int myid);
};

int client_register2(int fd, int myid, int broker_id);
int client_event_loop2(const int fd, const int myid, const struct event_handlers *hd, const void *context, const uint32_t interval);
int send_message_app(int fd, int myid, int receiver_id, void *payload, size_t size);
