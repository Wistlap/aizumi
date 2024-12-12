# Makefile

TARGETS     = target/release/raft-rs-broker

PERF        = perf stat -e cycles,instructions,cache-references,cache-misses,bus-cycles

# If not available perf command
# PERF        =

.PHONY: all clean test test-perf allow-perf test-e2e

all:
	cargo build --release

clean:
	rm -rf target

allow-perf:
	@sudo sh -c 'echo -1 >/proc/sys/kernel/perf_event_paranoid'
