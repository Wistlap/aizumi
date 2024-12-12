#include <stdlib.h>

#include "queue.h"
#include "statistics.h"

union status *get_status(int stat_num, struct mqueue_pool *queue_list) {
  union status *stat = malloc(sizeof(union status));
  switch(stat_num) {
    case QUEUE_STAT:
    stat->queue_stat = *queue_status(queue_list);
    break;
    case OUTPUT_LOG_AND_EXIT:
    stat->type = 2;
    break;
  }
  return stat;
}
