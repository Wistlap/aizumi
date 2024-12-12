# Makefile

TARGETS     = m-sender m-broker m-broker-tp m-receiver m-host_com m-trans_ctl m-amhs_com

CC          = gcc
CFLAGS      = -Wall -march=native -O3
LDLIBS      = -lpthread -lrt
PERF        = perf stat -e cycles,instructions,cache-references,cache-misses,bus-cycles

# For old compilers
# CFLAGS      = -Wall -march=native -O3 -std=c99

# If not available perf command
# PERF        =

SRCS        := $(wildcard *.c)
OBJS        := $(SRCS:.c=.o)
COMMON_OBJS := $(filter-out m-%.o,$(OBJS))

.PHONY: all clean test test-perf allow-perf test-e2e

.c.o:
	gcc $(CFLAGS) -c $<

all: $(TARGETS)

$(foreach EXE,$(TARGETS),$(eval $(EXE): $(COMMON_OBJS) $(EXE).o))

# Remove backslash cf. https://koturn.hatenablog.com/entry/2016/06/26/020000
$(foreach SRC,$(SRCS),$(eval $(subst \,,$(shell $(CC) -MM $(SRC)))))

clean:
	rm -f *.o
	rm -f $(TARGETS)
	- killall m-broker
	- killall m-broker-tp

test:
	@./m-broker & (sleep 1 && ./m-receiver -u 100 -c 100000)  & (sleep 2; ./m-sender -u 1 -c 100000 100)

test-perf: allow-perf test-e2e

allow-perf:
	@sudo sh -c 'echo -1 >/proc/sys/kernel/perf_event_paranoid'

test-e2e: $(TARGETS)
	@$(PERF) ./m-broker & (sleep 1 && ./m-receiver -u 100 -c 100000)  & (sleep 2; ./m-sender -u 1 -c 100000 100)
