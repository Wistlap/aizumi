#include "logger.h"
#include "message.h"
#include "network.h"

////////////////////////////////////////////////////////////////
/// private functions

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
// Returns new socket fd or -1 (on failue).
//
// int fd = net_connect("localhost", "3000");
//
int net_connect(const char *host, const char *service)
{
  return bind_or_connect(host, service, 0);
}

// Create TCP server (bind & listen)
// Returns new socket-fd or -1 (on failue).
//
// int fd = net_create_service("localhost", "3000");
// client_fd = net_accept(fd);
//
int net_create_service(const char *host, const char *service)
{
  return bind_or_connect(host, service, 1);
}

// Receive ack message from FD expecting EXPECTED_MSG_TYPE.
// Returns read-size:
//   MSG_TOTAL_LEN: succeed
//   0: no more data to read
//   otherwise: failure
//
// You can leave msg NULL if no need the content.
//
int net_recv_ack(int fd, struct message *msg, int expected_msg_type, int expected_orig_msg_id)
{
  struct message tmp_msg;

  if (!msg) {
    msg = &tmp_msg;
  }
  int n = net_recv_msg(fd, msg);
  int id = *((int*)msg->payload);
  if(expected_msg_type == MSG_STAT_RES) {
    return n;
  }

  if ((msg->hdr.msg_type != expected_msg_type) || (id != expected_orig_msg_id)) {
    logger_error("invalid ack: expected %s for id %d, received %s for id %d\n",
                 msg_type_to_string(expected_msg_type),
                 expected_orig_msg_id,
                 msg_type_to_string(msg->hdr.msg_type),
                 id);
    return -1;
  }
  return n;
}

// Receive message from FD.
// Returns read-size:
//   MSG_TOTAL_LEN: succeed
//   0: no more data to read
//   otherwise: failure
//
// You can leave msg NULL if no need the content.
//
int net_recv_msg(int fd, struct message *msg)
{
  struct message tmp_msg;

  if (!msg)
    msg = &tmp_msg;

  int n = receive_fixed_length(fd, msg, MSG_TOTAL_LEN);

  if (n != 0 && n != MSG_TOTAL_LEN) {
    logger_error("message size (%d) != MS_TOTAL_LEN (%ld) FD(%d)\n", n, MSG_TOTAL_LEN, fd);
  }

  logger_ttrace("i", " "); msg_fdump(logger_fp(LOG_TRACE), msg);
  return n;
}

// Send ack message corresponding to ORIG_MSG via FD.
// Returns
//   MSG_TOTAL_LEN: succeed
//   otherwise: failure
//
int net_send_ack(int fd, struct message *orig_msg, int myid)
{
  struct message msg;

  msg_fill_ack(&msg, orig_msg, myid);
  return net_send_msg(fd, &msg);
}

// Send message via FD.
// Returns
//   MSG_TOTAL_LEN: succeed
//   otherwise: failure
//
// You may want to use msg_fill to setup msg:
//    msg_fill_hdr(&msg, MSG_SEND_REQ, myid, receiver_id, 0);
//    msg_fill_sprintf(&msg, "Sending from sender %d", myid);
//    net_send_msg(fd, &msg);
//
int net_send_msg(int fd, struct message *msg)
{
  int n = send(fd, msg, MSG_TOTAL_LEN, 0);

  if (n != MSG_TOTAL_LEN)
    logger_error("send_msg failed (%d)\n", n);

  if (n < 0) {
    perror("send");
  }

  logger_ttrace("i", " "); msg_fdump(logger_fp(LOG_TRACE), msg);
  return n;
}
