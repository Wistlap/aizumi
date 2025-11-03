#include "logger.h"
#include "message.h"
#include "network.h"
#include "cli_parser.h"

////////////////////////////////////////////////////////////////
/// private functions


// make network result
// Returns struct network_result
//  status: result status
//  byte_size: size of sent/received bytes
//  hint: hint message for next action
struct network_result make_network_result(enum network_status status, int data, char *hint)
{
	struct network_result result;

	result.status = status;
	result.data = data;

	result.hint[0] = '\0';
	if (hint){
    strncpy(result.hint, hint, MSG_PAYLOAD_LEN - 1);
    result.hint[MSG_PAYLOAD_LEN - 1] = '\0';
  }

	return result;
};

/*
  Read fixed length from FD.
  Returns the amount of read (should be LENGHT).
  In error caes, return value will differ from LENGTH.

  XXX: Currently we simply mapped blob into struct message.
   It might need some deserialization process for interoperability.
*/
static int receive_fixed_length(int fd, void *buffer, int length) {
  int n, amount = 0;

  while ((n = read(fd, buffer + amount, length - amount)) > 0) {
    amount += n;
    if (amount == length) break;
  }
  return amount;
}

//
// Setup TCP socket interface for both servers and clients
// Returns new socket-fd or -1 (on failue).
//
// bind_or_connect("localhost", "3000", 1) ... perform bind & listen
// bind_or_connect("localhost", "3000", 0) ... perform connect
//
// HOST and SERVICE allows such like "127.0.0.1", "http".
//
//
static int bind_or_connect(const char *host, const char *service, int perform_bind)
{
  int fd, err;
  struct addrinfo hints, *res, *rp;

  memset(&hints, 0, sizeof(struct addrinfo));
  hints.ai_family = AF_INET;
  hints.ai_socktype = SOCK_STREAM;

  if ((err = getaddrinfo(host, service, &hints, &res))) {
    fprintf(stderr, "getaddrinfo: %s\n", gai_strerror(err));
    return err;
  }

  for (rp = res; rp != NULL; rp = rp->ai_next) {
    if ((fd = socket(rp->ai_family, rp->ai_socktype, rp->ai_protocol)) == -1)
      continue;

    int ret, on = 1;
    setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &on, sizeof(on));

    if (perform_bind) {
      setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &on, sizeof(on));
      ret = bind(fd, rp->ai_addr, rp->ai_addrlen);
    } else {
      ret = connect(fd, rp->ai_addr, rp->ai_addrlen);
    }

    if (ret != -1) break; /* Success */
    close(fd);
  }
  freeaddrinfo(res);

  if (rp == NULL) return -1;
  if (perform_bind && listen(fd, 100) != 0) return -1;
  return fd;
}

////////////////////////////////////////////////////////////////
/// public functions

// Wrapper function for accept(2)
//
//
int net_accept(int fd)
{
  int client_fd, on = 1;
  struct sockaddr_in sin_client;

  // sin_client and socklen are used only for logging
  memset(&sin_client, 0, sizeof(sin_client));
  socklen_t socklen = sizeof(sin_client);;
  client_fd = accept(fd, (struct sockaddr *)&sin_client, &socklen);
  logger_info("accepted connection from(%s) port(%d) FD(%d)\n", inet_ntoa(sin_client.sin_addr), ntohs(sin_client.sin_port), client_fd);
  setsockopt(client_fd, IPPROTO_TCP, TCP_NODELAY, &on, sizeof(on));
  return client_fd;
}

// Wrapper function for close(2)
//
int net_close(int fd)
{
  return close(fd);
}

// Create TCP connection
// Returns struct network_result include socket fd or -1 (on failue).
//
// struct network_result net_res = net_connect("localhost", "3000");
// if (net_res.status == NETWORK_STATUS_SUCCESS) {
//     int fd = net_res.data;
// }
struct network_result net_connect(const char *host, const char *service)
{
  int fd = bind_or_connect(host, service, 0);
  if (fd > 0) {
    return make_network_result(NETWORK_STATUS_SUCCESS, fd, NULL);
  }
  else {
    return make_network_result(NETWORK_STATUS_ERR_CONNECT, -1, "failed to connect");
  }
}

// Create TCP server (bind & listen)
// Returns struct network_result include socket fd or -1 (on failue).
//
// struct network_result net_res = net_create_service("localhost", "3000");
// if (net_res.status == NETWORK_STATUS_SUCCESS) {
//     int fd = net_res.data;
// }
// client_fd = net_accept(fd);
struct network_result net_create_service(const char *host, const char *service)
{
  int fd = bind_or_connect(host, service, 1);
  if (fd > 0) {
    return make_network_result(NETWORK_STATUS_SUCCESS, fd, NULL);
  }
  else {
    return make_network_result(NETWORK_STATUS_ERR_SOCKET, -1, "failed to create service");
  }
}

// Try to connect to multiple hosts recursively
// Returns socket fd or -1 (on failue).
//
int net_connect_recursive(const char *host, const char *service, const char **alt_hosts, int alt_host_count)
{
  struct network_result net_res = net_connect(host, service);
  int i = 0;
  while (net_res.status != NETWORK_STATUS_SUCCESS && i < alt_host_count) {
    logger_warn("trying to connect %s:%s ...\n",
                alt_hosts[i],  service);
    net_res = net_connect(alt_hosts[i], service);
    i++;
  }
  return net_res.data;
}

int net_reconnect(struct network_result old_net_res, struct cli_option opt) {
  int fd;
  struct network_result net_res = old_net_res;

  if (net_res.status == NETWORK_STATUS_ERR_DO_REDIRECT) {
    char *new_broker = net_res.hint;
    char *colon_pos = strchr(new_broker, ':');
    if (colon_pos) {
      *colon_pos = '\0';
      strncpy(opt.broker_host, new_broker, sizeof(opt.broker_host) - 1);
      opt.broker_host[sizeof(opt.broker_host) - 1] = '\0';
      strncpy(opt.broker_port, colon_pos + 1, sizeof(opt.broker_port) - 1);
      logger_info("redirecting to new broker %s:%s\n", opt.broker_host, opt.broker_port);
    } else {
      logger_error("invalid redirect hint: %s\n", new_broker);
    }
    net_res = net_connect(opt.broker_host, opt.broker_port);
    fd = net_res.data;
    if (fd < 0) {
      logger_error("failed to connect %s:%s\n",
                    opt.broker_host, opt.broker_port);
      exit(-1);
    }
  } else {
    fd = net_connect_recursive(opt.broker_host, opt.broker_port, (const char **)opt.broker_replicas,opt.broker_replicas_count);
    if (fd < 0) {
      logger_error("failed to connect %s:%s\n",
                    opt.broker_host, opt.broker_port);
      exit(-1);
    }
  }
  return fd;
}

// Receive ack message via FD.
// Returns struct network_result include read-size.
//
// You can leave msg NULL if no need the content.
//
struct network_result net_recv_ack(int fd, struct message *msg, int expected_msg_type, int expected_orig_msg_id)
{
  struct message tmp_msg;

  if (!msg) {
    msg = &tmp_msg;
  }

  struct network_result net_res = net_recv_msg(fd, msg);
  if (msg->hdr.msg_type == MSG_NACK) {
    net_res.status = NETWORK_STATUS_ERR_DO_REDIRECT;

    int hint_len = *((int*)msg->payload);
    char hint[hint_len + 1];
    // skip first 8 bytes (original payload length)
    int start_pos = 8;
    strncpy(hint, msg->payload + start_pos, hint_len);
    hint[hint_len] = '\0';

    strncpy(net_res.hint, hint, MSG_PAYLOAD_LEN - 1);
    net_res.hint[MSG_PAYLOAD_LEN - 1] = '\0';
    printf("Client: Received redirect hint: %s\n", net_res.hint);
    return net_res;
  }

  int id = *((int*)msg->payload);
  if(expected_msg_type == MSG_STAT_RES) {
    return net_res;
  }

  if ((msg->hdr.msg_type != expected_msg_type) || (id != expected_orig_msg_id)) {
    logger_error("invalid ack: expected %s for id %d, received %s for id %d\n",
                 msg_type_to_string(expected_msg_type),
                 expected_orig_msg_id,
                 msg_type_to_string(msg->hdr.msg_type),
                 id);
    net_res.status = NETWORK_STATUS_ERR_CONTENT;
    net_res.data = -1;
    return net_res;
  }
  return net_res;
}

// Receive message via FD.
// Returns struct network_result include read-size.
//
// You can leave msg NULL if no need the content.
//
struct network_result net_recv_msg(int fd, struct message *msg)
{
  struct message tmp_msg;

  if (!msg)
    msg = &tmp_msg;

  int n = receive_fixed_length(fd, msg, MSG_TOTAL_LEN);

  if (n != 0 && n != MSG_TOTAL_LEN) {
    logger_error("message size (%d) != MS_TOTAL_LEN (%ld) FD(%d)\n", n, MSG_TOTAL_LEN, fd);
    if (n == 0) {
      return make_network_result(NETWORK_STATUS_ERR_PEER_CLOSED, n, "peer closed connection");
    } else {
      return make_network_result(NETWORK_STATUS_ERR_RECV, n, "failed to receive message");
    }
  }

  logger_ttrace("i", " "); msg_fdump(logger_fp(LOG_TRACE), msg);
  return make_network_result(NETWORK_STATUS_SUCCESS, n, NULL);
}

// Send ack message corresponding to ORIG_MSG via FD.
// Returns struct network_result include sent-size.
//
struct network_result net_send_ack(int fd, struct message *orig_msg, int myid)
{
  struct message msg;

  msg_fill_ack(&msg, orig_msg, myid);
  return net_send_msg(fd, &msg);
}

// Send message via FD.
// Returns struct network_result include sent-size.
//
// You may want to use msg_fill to setup msg:
//    msg_fill_hdr(&msg, MSG_SEND_REQ, myid, receiver_id, 0);
//    msg_fill_sprintf(&msg, "Sending from sender %d", myid);
//    net_send_msg(fd, &msg);
//
struct network_result net_send_msg(int fd, struct message *msg)
{
  struct network_result net_res;

  int n = send(fd, msg, MSG_TOTAL_LEN, 0);

  if (n != MSG_TOTAL_LEN) {
    logger_error("send_msg failed (%d)\n", n);
    net_res = make_network_result(NETWORK_STATUS_ERR_SEND, n, "failed to send message");
  } else {
    // 送信先アドレスの出力
    struct sockaddr_in addr;
    socklen_t addrlen = sizeof(addr);
    if (getpeername(fd, (struct sockaddr *)&addr, &addrlen) == -1) {
        perror("getpeername");
        return net_res;
    }
    net_res = make_network_result(NETWORK_STATUS_SUCCESS, n, NULL);
  }

  if (n < 0) {
    perror("send");
  }

  logger_ttrace("i", " "); msg_fdump(logger_fp(LOG_TRACE), msg);
  return net_res;
};
