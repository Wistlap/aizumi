#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <unistd.h>

#include "logger.h"
#include "timer.h"

// XXX not thread-safe
uint64_t timer_clockcount_in_one_second = 0;

#define rdtsc_64(lower, upper)                              \
  __asm__ __volatile ("rdtsc" : "=a"(lower), "=d" (upper));

struct timer_storage {
  int count;
  int max_size;
  struct timer_record *records;
};

struct timer_record {
  uint32_t msg_id;
  uint32_t msg_type;
  uint64_t tsc;
};

struct timer_storage *timer_create_storage(size_t max_size)
{
  struct timer_storage *storage = malloc(sizeof(struct timer_storage));
  storage->count = 0;
  storage->max_size = max_size;
  storage->records = malloc(sizeof(struct timer_record) * max_size);

  return storage;
}

void timer_delete_storage(struct timer_storage *storage)
{
  free(storage->records);
  free(storage);
}

struct timer_storage *timer_append(struct timer_storage *storage, uint64_t tsc, uint32_t msg_id, uint32_t msg_type)
{
  if (storage->count < storage->max_size) {
    storage->records[storage->count].msg_id = msg_id;
    storage->records[storage->count].msg_type = msg_type;
    storage->records[storage->count].tsc = tsc;
    storage->count++;
  }
  return storage;
}

int timer_storage_nrecords(struct timer_storage *storage)
{
  return storage->count;
}

void timer_fdump(FILE *fp, struct timer_storage *storage)
{
  if (!fp) return;

  for (int i = 0; i < storage->count; i++) {
    fprintf(fp, "%d,%d,%d,%d,%" PRIu64 "\n",
            i+1,storage->count,
            storage->records[i].msg_id,
            storage->records[i].msg_type,
            storage->records[i].tsc
            );
  }
  fflush(fp);
}

void timer_dump(struct timer_storage *storage)
{
  timer_fdump(stdout, storage);
}

// Initialize and calibrate TSC counter.
uint64_t timer_setup_clock()
{
  timer_clockcount_in_one_second = timer_get_1sec_tsc();
  return timer_clockcount_in_one_second;
}

// Get clock-counter value using RDTSC
inline uint64_t timer_now()
{
  uint32_t tsc_l, tsc_u;

  rdtsc_64(tsc_l, tsc_u);
  return (uint64_t)tsc_u << 32 | tsc_l;
}

// Get clock-counter value per one second.
// XXX WIP
uint64_t timer_get_1sec_tsc()
{
  uint64_t clk, clk1, clk2;
  uint64_t nano, nano1, nano2;
  struct timespec ts1, ts2;

  clock_gettime(CLOCK_MONOTONIC, &ts1);
  clk1 = timer_now();
  sleep(1);
  clk2 = timer_now();
  clock_gettime(CLOCK_MONOTONIC, &ts2);

  nano1  = ((uint64_t)(ts1.tv_sec)) * 1000000000 + ((uint64_t)(ts1.tv_nsec));
  nano2  = ((uint64_t)(ts2.tv_sec)) * 1000000000 + ((uint64_t)(ts2.tv_nsec));

  nano = nano2 - nano1; // (ns)
  clk = clk2 - clk1;

  return clk * 1000000000 / nano;
}

// Get clock-counter dration from START to END
uint64_t timer_get_duration(uint64_t start, uint64_t end)
{
  return (end - start);
}

// Return duration between START and END in micro-sec.
//
// calibration using timer_setup_clock() is needed.
//
uint64_t timer_get_duration_us(uint64_t start, uint64_t end)
{
  if (timer_clockcount_in_one_second) {
    return (end - start) / (timer_clockcount_in_one_second / 1000000);

  } else {
    logger_error("time is not calibrated, call timer_setup_clock()");
    return 0;
  }
}

uint64_t getTickCount()
{
	struct timespec ts;
	if (clock_gettime(CLOCK_MONOTONIC, &ts) == -1)
	{
    logger_error("clock_gettime() fail");
		return 0;
	}
	return ((uint64_t)ts.tv_sec	* 1000ULL)        // sec
			  + ((uint64_t)ts.tv_nsec / 1000000ULL);  // nano sec
}

void busy_loop(const uint64_t wait_time, uint64_t* counter)
{
  const uint64_t comp_time = getTickCount() + wait_time;
  *counter = 0;
  while(getTickCount() < comp_time){
    (*counter)++;
  }
}

int millisleep(const uint64_t ms)
{
  const struct timespec req = {
    ms / 1000,                // seconds
    (ms % 1000) * 1000 * 1000 // nano seconds
  };
  return nanosleep(&req, NULL);
}
