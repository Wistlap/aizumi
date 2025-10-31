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
#include "cli_parser.h"

enum network_status {
	NETWORK_STATUS_SUCCESS = 0,
	NETWORK_STATUS_ERR_SOCKET,
	NETWORK_STATUS_ERR_CONNECT,
	NETWORK_STATUS_ERR_SEND,
	NETWORK_STATUS_ERR_RECV,
	NETWORK_STATUS_ERR_PEER_CLOSED,  // recv() = 0
	NETWORK_STATUS_ERR_TIMEOUT,
	NETWORK_STATUS_ERR_CONTENT,      // unexpected content
	NETWORK_STATUS_ERR_DO_REDIRECT,   // need to do redirection
};

struct network_result {
	enum network_status status; // network communication status
	int data; // data that depends on context (e.g., bytes sent/received, file descriptor, etc.)
	char hint[MSG_PAYLOAD_LEN]; // hint message for next action
};

struct network_result make_network_result(enum network_status status, int data, char *hint);

int net_accept(int fd);
int net_close(int fd);
struct network_result net_connect(const char *host, const char *service);
struct network_result net_create_service(const char *host, const char *service);
int net_connect_recursive(const char *host, const char *service, const char **alt_hosts, int alt_host_count);
int net_reconnect(struct network_result old_net_res, struct cli_option opt);
struct network_result net_recv_ack(int fd, struct message *msg, int expected_msg_type, int expected_orig_msg_id);
struct network_result net_recv_msg(int fd, struct message *msg);
struct network_result net_send_ack(int fd, struct message *orig_msg, int myid);
struct network_result net_send_msg(int fd, struct message *msg);
