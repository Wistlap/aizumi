#include <stdlib.h>
#include <string.h>
#include <pthread.h>
#include "queue.h"
#include "message.h"

struct mqueue *queue_create()
{
  struct mqueue* queue = malloc(sizeof(struct mqueue));

  if (queue == NULL) {
    return NULL;
  }
  queue->head = NULL;
  queue->tail = NULL;
  return queue;
}

struct item *item_create(struct message *message)
{
  struct item* item = malloc(sizeof(struct item));

  if (item == NULL) {
    return NULL;
  }
  struct message* msg = malloc(sizeof(struct message));
  struct message_header* msg_hdr = malloc(sizeof(struct message_header));
  memcpy(msg->payload, message->payload, MSG_PAYLOAD_LEN);
  msg_hdr->tot_len = message->hdr.tot_len;
  msg_hdr->msg_type = message->hdr.msg_type;
  msg_hdr->saddr = message->hdr.saddr;
  msg_hdr->daddr = message->hdr.daddr;
  msg_hdr->id = message->hdr.id;
  msg->hdr = *msg_hdr;
  item->msg = msg;
  item->next = NULL;
  return item;
}

void queue_free(struct mqueue *queue)
{
  struct item *current = queue->head;
  struct item *next;

  while (current != NULL)
  {
    next = current->next;
    free(current);
    current = next;
  }
  free(queue);
}

struct mqueue *queue_add(struct mqueue *queue, struct item *item)
{
  if (queue->head == NULL) {
    queue->head = item;
    queue->tail = item;
    item->next = NULL;
  } else {
    queue->tail->next = item;
    item->next = NULL;
    queue->tail = item;
  }
  return queue;
}

struct mqueue *queue_del(struct mqueue *queue)
{
  if (queue->head == NULL) {
    return NULL;
  } else {
    struct item *item = queue->head;
    queue->head = item->next;
    free(item);
  }
  return queue;
}

struct mqueue *queue_del_byid(struct mqueue *queue, int id)
{
  struct item *qp = queue->head;
  struct item *qp_prev = NULL;

  while (qp) {
    if (qp->msg->hdr.id == id) {
      if (qp == queue->head)
        queue->head = qp->next;
      if (qp == queue->tail)
        queue->tail = qp_prev;
      if (qp_prev)
        qp_prev->next = qp->next;

      free(qp);
      return queue;
    }
    qp_prev = qp;
    qp = qp->next;
  }
  return queue;
}

bool queue_is_empty(struct mqueue *queue)
{
  return (queue->head == NULL);
}

void *queue_head(struct mqueue *queue)
{
  return queue->head;
}

void *queue_tail(struct mqueue *queue)
{
  return queue->tail;
}

struct mqueue_pool *queue_pool_create()
{
  struct mqueue_pool *queue_pool = malloc(sizeof(struct mqueue_pool)* QUEUE_NUMS);

  if (queue_pool == NULL) {
    return NULL;
  }
  int i;

  for (i = 0; i < QUEUE_NUMS; i++) {
    queue_pool[i].addr = 0;
    queue_pool[i].fd = 0;
    queue_pool[i].queue = NULL;
    queue_pool[i].delivered_queue = NULL;
  }
  return queue_pool;
}

int queue_pool_add(struct mqueue_pool *queue_pool, int addr, int fd, struct mqueue *queue, struct mqueue *delivered_queue)
{
  int i, index;
  pthread_mutex_t mp = PTHREAD_MUTEX_INITIALIZER;

  for (i = 0; i < QUEUE_NUMS; i++) {
    index = (addr+i)%QUEUE_NUMS;
    if (queue_pool[index].addr == 0) {
      queue_pool[index].addr = addr;
      queue_pool[index].fd = fd;
      queue_pool[index].mp = mp;
      queue_pool[index].queue = queue;
      queue_pool[index].delivered_queue = delivered_queue;
      break;
    }
  }
  return index;
}

struct mqueue_pool *queue_pool_add_ptr(struct mqueue_pool *queue_pool, int addr, int fd, struct mqueue *queue, struct mqueue *delivered_queue)
{
  int index = queue_pool_add(queue_pool, addr, fd, queue, delivered_queue);
  return &queue_pool[index];
}


struct mqueue_pool *queue_pool_del(struct mqueue_pool *queue_pool, int addr, int fd)
{
  int i;

  for (i = 0; i < QUEUE_NUMS; i++) {
    if (queue_pool[i].addr == addr) {
      queue_free(queue_pool[i].queue);
    }
  }
  return queue_pool;
}

void queue_pool_free(struct mqueue_pool *queue_pool)
{
  int i;

  for (i = 0; i < QUEUE_NUMS; i++) {
    queue_free(queue_pool[i].queue);
  }
  free(queue_pool);
}

int queue_search(struct mqueue_pool *queue_pool, int addr)
{
  int i, index;
  for (i = 0; i < QUEUE_NUMS; i++) {
    index = (addr+i)%QUEUE_NUMS;
    if (queue_pool[index].addr == addr) {
      return index;
    }
  }
  return -1;
}

struct mqueue_pool *queue_search_ptr(struct mqueue_pool *queue_pool, int addr)
{
  int index = queue_search(queue_pool, addr);
  if (index < 0) {
    return NULL;
  } else {
    return &queue_pool[index];
  }
}

struct mqueue_stat *queue_status(struct mqueue_pool *queue_pool)
{
  struct mqueue_stat *stat = malloc(sizeof(struct mqueue_stat));
  int sum_messages = 0;
  int max_messages = 0;
  int sum_queues = 0;
  int i;
  for (i = 0; i < QUEUE_NUMS; i++) {
    if (queue_pool[i].addr != 0) {
      struct item *next = queue_pool[i].queue->head;
      int messages_num = 0;
      while (next) {
        next = next->next;
        messages_num++;
      }
      sum_messages += messages_num;
      if (messages_num > max_messages) {
        max_messages = messages_num;
      }
      sum_queues++;
    }
  }
  stat->sum_messages = sum_messages;
  stat->max_messages = max_messages;
  stat->sum_queues = sum_queues;
  return stat;
}
