# SUBDIRS := $(wildcard */.)
SUBDIRS := broker client

all: $(SUBDIRS)

$(SUBDIRS):
	$(MAKE) -C $@

.PHONY: all $(SUBDIRS)

# -d: debug level
# options after -- will be passed to to raft-rs-broker
#
test:
	- killall rast-rs-broker
	scripts/autotest_all.sh -b broker/target/release/raft-rs-broker -r 0,10,20 -s 1,10,20 -c 100000 -d 3 -l "log/$$(date +%Y%m%d-%H%M%S)/raft-rs-broker-%s-%r-%t-%c-%g-$$(date +%Y%m%d-%H%M%S).log" -- -t 1 -g 5

# sender, receiver : 10,10, raft nodes: 1,2,..,10
raft-test10:
	- killall rast-rs-broker
	scripts/autotest_all.sh -b broker/target/release/raft-rs-broker -r 10 -s 10 -c 100000 -d 3 -l "log/$$(date +%Y%m%d-%H%M%S)/raft-rs-broker-%s-%r-%t-%c-%g-$$(date +%Y%m%d-%H%M%S).log" -- -t 1 -g 1-10

# sender, receiver : 1,1, raft nodes: 1,2,..,10
raft-test1:
	- killall rast-rs-broker
	scripts/autotest_all.sh -b broker/target/release/raft-rs-broker -r 1 -s 1 -c 100000 -d 3 -l "log/$$(date +%Y%m%d-%H%M%S)/raft-rs-broker-%s-%r-%t-%c-%g-$$(date +%Y%m%d-%H%M%S).log" -- -t 1 -g 1-10

stat:
	@for log in log/* ;do \
	  if [ ! -f $$log/*.stat ]; then \
	    echo scripts/csv2stat_all.sh $$log ;\
	    scripts/csv2stat_all.sh $$log ;\
	  fi \
	done

plot:
	@for logdir in log/* ;do \
	  (cd $$logdir; ../../scripts/stat2graph.py . -tb) ;\
	done
