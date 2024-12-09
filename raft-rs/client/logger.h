#pragma once

#include <stdio.h>

// Supporsed to be filled in logger.c
extern FILE *logger_default_stream;
extern char logger_default_progname[128];
extern int logger_current_level;

enum { LOG_TRACE, LOG_DEBUG, LOG_INFO, LOG_WARN, LOG_ERROR, LOG_FATAL };

int logger_set_current_level(int level);
int logger_ok(int level);
FILE* logger_fp(int level);
void logger_timestamp();

void logger_setup_log_file(const char *pathname, const char *prog_name);
void logger_setup_pid_file(const char *pathname, const char *prog_name);
void logger_tsc(int id, int msg_type, long int tsc);
void logger_str(char *str);

#define logger_lprintf(level, hdr, fmt, ...)                            \
  if (logger_ok(level))  {                                              \
    fprintf(logger_default_stream, "%s: %s:%s:%s(L%u): " fmt,           \
            hdr, logger_default_progname, __FILE__, __func__, __LINE__, ##__VA_ARGS__); \
    fflush(logger_default_stream);                                      \
  }

#define logger_ltprintf(level, pre, fmt, ...)                           \
  if (logger_ok(level))  {                                              \
    fprintf(logger_default_stream, pre);                                \
    logger_timestamp();                                                 \
    fprintf(logger_default_stream, fmt, ##__VA_ARGS__);                 \
    fflush(logger_default_stream);                                      \
  }

#define logger_printf(fmt, ...) logger_lprintf(100,       ""       fmt, ##__VA_ARGS__)
#define logger_trace(fmt, ...)  logger_lprintf(LOG_TRACE, "Trace", fmt, ##__VA_ARGS__)
#define logger_debug(fmt, ...)  logger_lprintf(LOG_DEBUG, "Debug", fmt, ##__VA_ARGS__)
#define logger_info(fmt, ...)   logger_lprintf(LOG_INFO,  "Info",  fmt, ##__VA_ARGS__)
#define logger_warn(fmt, ...)   logger_lprintf(LOG_WARN,  "Warn",  fmt, ##__VA_ARGS__)
#define logger_error(fmt, ...)  logger_lprintf(LOG_ERROR, "Error", fmt, ##__VA_ARGS__)
#define logger_fatal(fmt, ...)  logger_lprintf(LOG_FATAL, "Fatal", fmt, ##__VA_ARGS__)

#define logger_tprintf(pre, fmt, ...) logger_ltprintf(100,       pre, fmt, ##__VA_ARGS__)
#define logger_ttrace(pre, fmt, ...)  logger_ltprintf(LOG_TRACE, pre, fmt, ##__VA_ARGS__)
#define logger_tdebug(pre, fmt, ...)  logger_ltprintf(LOG_DEBUG, pre, fmt, ##__VA_ARGS__)
#define logger_tinfo(pre, fmt, ...)   logger_ltprintf(LOG_INFO,  pre, fmt, ##__VA_ARGS__)
#define logger_twarn(pre, fmt, ...)   logger_ltprintf(LOG_WARN,  pre, fmt, ##__VA_ARGS__)
#define logger_terror(pre, fmt, ...)  logger_ltprintf(LOG_ERROR, pre, fmt, ##__VA_ARGS__)
#define logger_tfatal(pre, fmt, ...)  logger_ltprintf(LOG_FATAL, pre, fmt, ##__VA_ARGS__)
