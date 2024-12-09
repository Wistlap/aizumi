#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>
#include <time.h>
#include <sys/time.h>
#include "logger.h"

FILE *logger_default_stream = NULL;
char logger_default_progname[128] = __FILE__;
int logger_current_level = LOG_WARN;

int logger_set_current_level(int level)
{
  logger_current_level = level;
  return logger_current_level;
}

int logger_ok(int level)
{
  return (level >= logger_current_level);
}

FILE* logger_fp(int level)
{
  if (logger_ok(level)) {
    return logger_default_stream;
  } else {
    return NULL;
  }
}

static FILE *logger_set_default_stream(const char *pathname)
{
  if (pathname == NULL ||
      strcmp(pathname, "") == 0 ||
      strcmp(pathname, "stdout") == 0) {

    logger_default_stream = stdout;

  } else if (strcmp(pathname, "stderr") == 0) {
    logger_default_stream = stderr;

  } else {

    logger_default_stream = fopen(pathname, "a");
  }

  return logger_default_stream;
}

static int logger_create_pid_file(const char *pathname)
{
  FILE *fp = fopen(pathname, "w");
  if (!fp) return -1;
  fprintf(fp, "%d\n", getpid());
  return fclose(fp) == 0 ? 0 : -1;
}

void logger_setup_log_file(const char *pathname, const char *prog_name)
{
  if (!logger_set_default_stream(pathname)) {
    fprintf(stderr, "%s: failed to open log-file: %s\n", prog_name, pathname);
    exit(-1);
  }
  strcpy(logger_default_progname, prog_name);
}

void logger_setup_pid_file(const char *pathname, const char *prog_name)
{
  if (pathname == NULL || strcmp(pathname, "") == 0) {
    return;

  } else if (logger_create_pid_file(pathname) < 0) {
    fprintf(stderr, "%s: failed to create pid-file: %s\n", prog_name, pathname);
    exit(-1);
  }
}

void logger_timestamp()
{
  //time_t now = time(NULL);
  //struct tm tmp;
  char buf[100];

  //localtime_r(&now, &tmp);
  //strftime(buf, sizeof(buf), "%Y-%m-%d %H:%M:%S", &tmp);

  struct tm *now;
  struct timeval total_usec;

  if(gettimeofday(&total_usec, NULL) == -1) {
      fprintf(stderr,"gettimeofday error");
      return;
  }

  now = localtime(&total_usec.tv_sec);
  sprintf(buf, "%d-%d-%d %d:%d:%d.%06ld ",
               now->tm_year+1900,
               now->tm_mon+1,
               now->tm_mday,
               now->tm_hour,
               now->tm_min,
               now->tm_sec,
               total_usec.tv_usec);

  if (logger_default_stream) {
    fprintf(logger_default_stream, "%s", buf);
  } else {
    fprintf(stdout, "%s", buf);
  }
}

void logger_tsc(int id, int msg_type, long int tsc)
{
  if (logger_default_stream) {
    fprintf(logger_default_stream, "%d, %d, %ld\n", id, msg_type, tsc);
    fflush(logger_default_stream);
  } else {
    fprintf(stdout, "%d, %d, %ld\n", id, msg_type, tsc);
  }
}

void logger_str(char *str)
{
  if (logger_default_stream) {
    fprintf(logger_default_stream, "%s\n", str);
    fflush(logger_default_stream);
  } else {
    fprintf(stdout, "%s\n", str);
  }
}
