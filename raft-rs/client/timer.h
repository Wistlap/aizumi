#pragma once

#include <stdint.h>

// XXX not thread-safe
extern uint64_t timer_clockcount_in_one_second;

struct timer_storage *timer_create_storage(size_t max_size);
void timer_delete_storage(struct timer_storage *storage);
struct timer_storage *timer_append(struct timer_storage *storage, uint64_t tsc, uint32_t msg_id, uint32_t msg_type);
void timer_fdump(FILE *fp, struct timer_storage *storage);
int timer_storage_nrecords(struct timer_storage *storage);

uint64_t timer_now();
uint64_t timer_get_1sec_tsc();
uint64_t timer_get_duration(uint64_t start, uint64_t end);
uint64_t timer_get_duration_us(uint64_t start, uint64_t end);
uint64_t timer_setup_clock();

uint64_t getTickCount();
void busy_loop(const uint64_t wait_time, uint64_t* counter);
int millisleep(const uint64_t ms);
