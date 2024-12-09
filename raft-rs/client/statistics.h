union status {
  int type;
  struct mqueue_stat queue_stat;
};

#define QUEUE_STAT 1
#define OUTPUT_LOG_AND_EXIT 2

union status *get_status(int stat_num, struct mqueue_pool *queue_list);
