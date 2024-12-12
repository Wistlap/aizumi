#pragma once

#include <netdb.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <stdio.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <unistd.h>

#include "message.h"

int net_accept(int fd);
int net_close(int fd);
int net_connect(const char *host, const char *service);
int net_create_service(const char *host, const char *service);
int net_recv_ack(int fd, struct message *msg, int expected_msg_type, int expected_orig_msg_id);
int net_recv_msg(int fd, struct message *msg);
int net_send_ack(int fd, struct message *orig_msg, int myid);
int net_send_msg(int fd, struct message *msg);
