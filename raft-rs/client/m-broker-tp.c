#include <stdio.h>
#include <sys/epoll.h>
#include <errno.h>

#include "cli_parser.h"
#include "logger.h"
#include "message.h"
#include "network.h"
#include "pthread.h"
#include "queue.h"
#include "timer.h"
#include "statistics.h"

#define PROG_NAME "m-broker-tp"
#define MAX_EVENTS 5
#define MAX_COUNT 3000000

struct th_arg {
  int fd;
  int id;
};

struct mqueue_pool* queue_pool_list;
int timeout;
pthread_mutex_t hello_mp = PTHREAD_MUTEX_INITIALIZER;
pthread_mutex_t log_mp = PTHREAD_MUTEX_INITIALIZER;
int epfd;
int accept_fd;

void *treat_client(void *arg_ptr)
{
  struct th_arg* args_ptr = (struct th_arg*)arg_ptr;
  int client_fd;
  int client_id = -1;
  int myid = args_ptr->id;
  int i, n, count = 0;
  int nfd;
  int stat_type;
  struct message msg;
  struct timer_storage *tscs = timer_create_storage(MAX_COUNT);
  union status *stat;
  struct epoll_event ev;
  struct epoll_event events[MAX_EVENTS];
  free(arg_ptr);

  while (1) {
    nfd = epoll_wait(epfd, events, MAX_EVENTS, timeout);
    // Timeout from epoll_wait
    /*
    if (nfd == 0) {

      struct mqueue_pool *dqueue_ptr = queue_search_ptr(queue_pool_list, client_id);

      if (dqueue_ptr && !queue_is_empty(dqueue_ptr->queue) && queue_is_empty(dqueue_ptr->delivered_queue)) {
        int id = dqueue_ptr->queue->head->msg->hdr.id;
        msg_fill_hdr(dqueue_ptr->queue->head->msg, MSG_PUSH_REQ,
                     dqueue_ptr->queue->head->msg->hdr.saddr,
                     dqueue_ptr->queue->head->msg->hdr.daddr,
                     id);

        timer_append(tscs, timer_now(), id, MSG_PUSH_REQ);
        net_send_msg(client_fd, dqueue_ptr->queue->head->msg);

        // Enqueue item to delivered queue
        queue_add(dqueue_ptr->delivered_queue, item_create(dqueue_ptr->queue->head->msg));

        // Dequeue item from message queue
        pthread_mutex_lock(&(dqueue_ptr->mp));
        queue_del(dqueue_ptr->queue);
        pthread_mutex_unlock(&(dqueue_ptr->mp));

        net_recv_ack(client_fd, NULL, MSG_PUSH_ACK, id);
      }
      count++;
      continue;
    }*/
      for (i=0; i<nfd; i++) {
        client_fd = events[i].data.fd;
      if (client_fd == accept_fd) {
        client_fd = net_accept(accept_fd);

        if (client_fd < 0) {
          logger_error("failed to accept\n");
          exit(-1);
        }
        ev.events = EPOLLIN | EPOLLONESHOT;
        ev.data.fd = client_fd;
        if (epoll_ctl(epfd, EPOLL_CTL_ADD, client_fd, &ev) != 0)
        {
          logger_error("epoll_ctl: %d\n", errno);
        }
        ev.events = EPOLLIN | EPOLLONESHOT;
        ev.data.fd = accept_fd;
        if (epoll_ctl(epfd, EPOLL_CTL_MOD, accept_fd, &ev) != 0)
        {
          logger_error("epoll_ctl: %d\n", errno);
        }
      } else {

        struct network_result net_res = net_recv_msg(client_fd, &msg);
        if ((n = net_res.data) != MSG_TOTAL_LEN) {
//          break;
//closeの処理をここで入れる キューからもfdを消す
          close(client_fd);
          logger_info("connection closed from peer (count: %d last-read: %d, tsc_count: %d)\n", count, n, timer_storage_nrecords(tscs));
          pthread_mutex_lock(&log_mp);
          timer_fdump(logger_fp(LOG_WARN), tscs); // logger_fp(LOG_INFO)
          pthread_mutex_unlock(&log_mp);
          continue;
        }

        switch (msg.hdr.msg_type) {

        case MSG_SEND_REQ: // (1) senver -> broker (+payload)
          timer_append(tscs, timer_now(), msg.hdr.id, MSG_SEND_REQ);

          int id = msg.hdr.id;
          msg.hdr.id = message_get_id();

          struct mqueue_pool *dqueue_ptr = queue_search_ptr(queue_pool_list, msg.hdr.daddr);
          if (!dqueue_ptr) {
            pthread_mutex_lock(&hello_mp);
            dqueue_ptr = queue_pool_add_ptr(queue_pool_list, msg.hdr.daddr, -1, queue_create(), queue_create());
            pthread_mutex_unlock(&hello_mp);
          }
          pthread_mutex_lock(&(dqueue_ptr->mp));
          queue_add(dqueue_ptr->queue, item_create(&msg));
          pthread_mutex_unlock(&(dqueue_ptr->mp));
          msg.hdr.id = id;
          net_send_ack(client_fd, &msg, myid);
          ev.events = EPOLLIN | EPOLLONESHOT;
          ev.data.fd = client_fd;
          if (epoll_ctl(epfd, EPOLL_CTL_MOD, client_fd, &ev) != 0)
          {
            logger_error("msg_type: %d\n",msg.hdr.msg_type);
            logger_error("epoll_ctl: %d\n", errno);
          }
          if (dqueue_ptr->fd != 0) {
            if (pthread_mutex_trylock(&(dqueue_ptr->mp)) == 0) {
              if (!queue_is_empty(dqueue_ptr->queue) && queue_is_empty(dqueue_ptr->delivered_queue)) {
                int receiver_fd = dqueue_ptr->fd;
                if (epoll_ctl(epfd, EPOLL_CTL_DEL, receiver_fd, NULL) != 0)
                {
                  logger_error("epoll_ctl: %d\n", errno);
                }
                msg_fill_hdr(dqueue_ptr->queue->head->msg,
                             MSG_PUSH_REQ,
                             dqueue_ptr->queue->head->msg->hdr.saddr,
                             dqueue_ptr->queue->head->msg->hdr.daddr,
                             dqueue_ptr->queue->head->msg->hdr.id);

                int id = dqueue_ptr->queue->head->msg->hdr.id;

                timer_append(tscs, timer_now(), id, MSG_PUSH_REQ);
                net_send_msg(receiver_fd, dqueue_ptr->queue->head->msg);

                queue_add(dqueue_ptr->delivered_queue, item_create(dqueue_ptr->queue->head->msg));

                queue_del(dqueue_ptr->queue);

                net_recv_ack(receiver_fd, NULL, MSG_PUSH_ACK, id);

              pthread_mutex_unlock(&(dqueue_ptr->mp));
                ev.events = EPOLLIN | EPOLLONESHOT;
                ev.data.fd = receiver_fd;
                if (epoll_ctl(epfd, EPOLL_CTL_ADD, receiver_fd, &ev) != 0)
                {
                  logger_error("epoll_ctl: %d\n", errno);
                }
                break;
              }
              pthread_mutex_unlock(&(dqueue_ptr->mp));
            }
          }
          count++;
          break;

        case MSG_RECV_REQ: // (3) receiver -> broker
          break;

        case MSG_FREE_REQ: // (5) receiver -> broker
          net_send_ack(client_fd, &msg, myid);
          dqueue_ptr = queue_search_ptr(queue_pool_list, msg.hdr.saddr);
          queue_del_byid(dqueue_ptr->delivered_queue, msg.hdr.id);
          if (dqueue_ptr) {
            if (pthread_mutex_trylock(&(dqueue_ptr->mp)) == 0) {
              if (!queue_is_empty(dqueue_ptr->queue) && queue_is_empty(dqueue_ptr->delivered_queue)) {
                msg_fill_hdr(dqueue_ptr->queue->head->msg,
                             MSG_PUSH_REQ,
                             dqueue_ptr->queue->head->msg->hdr.saddr,
                             dqueue_ptr->queue->head->msg->hdr.daddr,
                             dqueue_ptr->queue->head->msg->hdr.id);

                int id = dqueue_ptr->queue->head->msg->hdr.id;

                timer_append(tscs, timer_now(), id, MSG_PUSH_REQ);
                net_send_msg(client_fd, dqueue_ptr->queue->head->msg);

                queue_add(dqueue_ptr->delivered_queue, item_create(dqueue_ptr->queue->head->msg));

                queue_del(dqueue_ptr->queue);

                net_recv_ack(client_fd, NULL, MSG_PUSH_ACK, id);
              pthread_mutex_unlock(&(dqueue_ptr->mp));
                ev.events = EPOLLIN | EPOLLONESHOT;
                ev.data.fd = client_fd;
                if (epoll_ctl(epfd, EPOLL_CTL_MOD, client_fd, &ev) != 0) {
                  logger_error("msg_type: %d\n",msg.hdr.msg_type);
                  logger_error("epoll_ctl: %d\n", errno);
                }
                count++;
                break;
              }
              pthread_mutex_unlock(&(dqueue_ptr->mp));
            } else {
              continue;
            }
          }
          count++;
          break;

        case MSG_HELO_REQ: // (9) client -> broker
          net_send_ack(client_fd, &msg, myid);
          client_id = msg.hdr.saddr;
          logger_info("client_id: %d hello\n", client_id);
          pthread_mutex_lock(&hello_mp);
          dqueue_ptr = queue_search_ptr(queue_pool_list, msg.hdr.saddr);

          if (!dqueue_ptr) {
            queue_pool_add(queue_pool_list, msg.hdr.saddr, client_fd, queue_create(), queue_create());
          } else {
            dqueue_ptr->fd = client_fd;
          }
          pthread_mutex_unlock(&hello_mp);
          ev.events = EPOLLIN | EPOLLONESHOT;
          ev.data.fd = client_fd;
          if (epoll_ctl(epfd, EPOLL_CTL_MOD, client_fd, &ev) != 0)
          {
            logger_error("msg_type: %d\n",msg.hdr.msg_type);
            logger_error("epoll_ctl: %d\n", errno);
          }
          break;

        case MSG_STAT_REQ: // (11) client -> broker
        logger_info("stat_req\n");
        memcpy(&stat_type, msg.payload, sizeof(stat_type));
        stat = get_status(stat_type, queue_pool_list);
        switch (stat_type) {
          case QUEUE_STAT:
            msg_fill_hdr(&msg, MSG_STAT_RES,
                         msg.hdr.saddr,
                         msg.hdr.daddr,
                         msg.hdr.id);
            memcpy(&msg.payload, stat, sizeof(union status));
            break;
          case OUTPUT_LOG_AND_EXIT:
            pthread_mutex_lock(&log_mp);
            timer_fdump(logger_fp(LOG_FATAL), tscs);
            pthread_mutex_unlock(&log_mp);
            net_send_msg(client_fd, &msg);
            ev.events = EPOLLIN | EPOLLONESHOT;
            ev.data.fd = client_fd;
            if (epoll_ctl(epfd, EPOLL_CTL_MOD, client_fd, &ev) != 0)
            {
              logger_info("msg_type: %d\n",msg.hdr.msg_type);
              logger_error("epoll_ctl:%d\n", errno);
            }
            pthread_exit(NULL);
            break;
          default:
            // not reach
            break;
        }
        net_send_msg(client_fd, &msg);
        ev.events = EPOLLIN | EPOLLONESHOT;
        ev.data.fd = client_fd;
        if (epoll_ctl(epfd, EPOLL_CTL_MOD, client_fd, &ev) != 0)
        {
          logger_error("msg_type: %d\n",msg.hdr.msg_type);
          logger_error("epoll_ctl: %d\n", errno);
        }
        break;

        default:
          logger_warn("invalid message type %s (%d) ignored.",
                      msg_type_to_string(msg.hdr.msg_type),
                      msg.hdr.msg_type
                      );
          break;
        }
      }
    }
//    if (count % 10000 == 0) logger_info("%d messages processed\n", count);
  }
  close(client_fd);
  logger_info("connection closed from peer (count: %d last-read: %d, tsc_count: %d)\n", count, n, timer_storage_nrecords(tscs));
  pthread_mutex_lock(&log_mp);
//  timer_fdump(logger_fp(LOG_INFO), tscs);
  pthread_mutex_unlock(&log_mp);
  return NULL;
}

void perform_service(int fd, int myid, int threads_num)
{
  struct epoll_event ev;
  pthread_t th[threads_num];
  accept_fd = fd;

  epfd = epoll_create(MAX_EVENTS);
  if (epfd < 0)
  {
    logger_error("epoll_create");
    return;
  }
  memset(&ev, 0, sizeof ev);
  ev.events = EPOLLIN | EPOLLONESHOT;
  ev.data.fd = accept_fd;
  if (epoll_ctl(epfd, EPOLL_CTL_ADD, accept_fd, &ev) != 0)
  {
    logger_error("epoll_ctl: %d\n", errno);
  }

  int i;
  for (i=0; i<threads_num; i++) {
    struct th_arg *thread_args = malloc(sizeof(struct th_arg));
    thread_args->id = myid + i;
    thread_args->fd = fd;
    if (pthread_create(&th[i], NULL, treat_client, (void *)thread_args) < 0) {
      logger_error("failed to pthread_crate\n");
      exit(-1);
    }
  }
  for (i=0; i<threads_num; i++) {
    pthread_join(th[i], NULL);
  }
}

void usage_and_exit(int status)
{
  fprintf(stderr, "Usage: %s [-d LOG_LEVEL] [-b BROKER:PORT] [-c MSGS]\
 [-l LOG-FILE] [-n THREADS_NUM] [-p PID-FILE] [-u MYID]\
\n", PROG_NAME);
  exit(status);
}

int main(int argc, char *argv[])
{
  struct cli_option opt = CLI_OPTION_DEFAULT; // clone default
  opt.broker_timeout = -1; // default timeout
  opt.myid = MSG_ADDR_BROKER; // default broker address

  if (cli_parse_option(argc, argv, &opt) < 0 || opt.receivers[0] != -1)
    usage_and_exit(-1);

  logger_set_current_level(opt.debug_level);
  logger_setup_log_file(opt.log_file, PROG_NAME);
  logger_setup_pid_file(opt.pid_file, PROG_NAME);
  timeout = opt.broker_timeout;
  cli_fdump(logger_fp(LOG_DEBUG), &opt);

  logger_str("id, msg_type, tsc");

  struct network_result net_res = net_create_service(opt.broker_host, opt.broker_port);
  int fd = net_res.data;
  queue_pool_list = queue_pool_create();
  if (fd < 0) {
    logger_error("failed to bind %s:%s\n",
                 opt.broker_host, opt.broker_port);
    exit(-1);
  }

  logger_tinfo("* ", " %s started.\n", PROG_NAME);
  perform_service(fd, opt.myid, opt.broker_threads);
  return 0;
}
