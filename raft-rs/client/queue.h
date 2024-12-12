#include <stdbool.h>

#define QUEUE_NUMS 10000
#define QUEUE_BIN_NUMS 1
#define QUEUE_SEARCH_ASCEND 1

struct mqueue {
  struct item *head;
  struct item *tail;
};

struct item {
  struct message *msg;
  struct item *next;
};

struct mqueue_pool {
  int addr;
  int fd;
  pthread_mutex_t mp;
  struct mqueue *queue;
  struct mqueue *delivered_queue;
};

struct mqueue_stat {
  int stat_type;
  int sum_messages; //queueに入っているメッセージの合計数
  int sum_queues; //addrが登録されているキューの合計数
  int max_messages; //一番メッセージが入っているキューのメッセージの合計数
};



struct mqueue *queue_create();
struct item *item_create(struct message *message);
void queue_free(struct mqueue *queue);
struct mqueue *queue_add(struct mqueue *queue, struct item *item);
struct mqueue *queue_del(struct mqueue *queue);
struct mqueue *queue_del_byid(struct mqueue *queue, int id);
void *queue_head(struct mqueue *queue);
void *queue_tail(struct mqueue *queue);
struct mqueue_pool *queue_pool_create();
int queue_pool_add(struct mqueue_pool *queue_pool, int addr, int fd, struct mqueue *queue, struct mqueue *delivered_queue);
struct mqueue_pool *queue_pool_del(struct mqueue_pool *queue_pool, int addr, int fd);
void queue_pool_free(struct mqueue_pool *queue_pool);
struct mqueue_pool *queue_search_ptr(struct mqueue_pool *queue_pool, int addr);
bool queue_is_empty(struct mqueue *queue);
struct mqueue_pool *queue_pool_add_ptr(struct mqueue_pool *queue_pool, int addr, int fd, struct mqueue *queue, struct mqueue *delivered_queue);
struct mqueue_stat *queue_status(struct mqueue_pool *queue_pool);
