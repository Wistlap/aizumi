#!/bin/bash
url="http://localhost:8080/"
url5="$url $url $url $url $url"
url10="$url5 $url5"
url50="$url10 $url10 $url10 $url10 $url10"
for i in $(seq 1 2000); do
curl --parallel --parallel-immediate --parallel-max 50 -XPOST -s -d '{"msg_type": "MSG_HELO_REQ","saddr": 100,"daddr": 5000,"id": 1,"payload": "hello"}' -H "Content-Type: application/json"  $url50> /dev/null
done
