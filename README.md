# aizumi

## API
``` c
#define MSG_SEND_REQ  1 // sender -> broker (+payload)
#define MSG_SEND_ACK  2 // broker -> sender
#define MSG_RECV_REQ  3 // receiver -> broker
#define MSG_RECV_ACK  4 // broker -> receiver (+payload)
#define MSG_FREE_REQ  5 // receiver -> broker
#define MSG_FREE_ACK  6 // broker -> receiver
#define MSG_PUSH_REQ  7 // broker -> receiver (+payload)
#define MSG_PUSH_ACK  8 // receiver -> broker
#define MSG_HELO_REQ  9 // client -> broker
#define MSG_HELO_ACK 10 // broker -> client
#define MSG_STAT_REQ 11 // client -> broker
#define MSG_STAT_RES 12 // broker -> client
#define MSG_GBYE_REQ 13 // client -> broker
#define MSG_GBYE_ACK 14 // broker-> client
```
* MSG_PUSH_REQ と MSG_PUSH_ACK はサポートしない．
* MSG_GBYE_REQ と MSG_GBYE_ACK は JSON API のために新設された．
* _ACK と _RES は HTTP のレスポンスとして即座に返される．
* 上記を JSON 形式で POST することでリクエストを構成する．

## Payload
この章の JSON の例の id は以下のようになっている．
* sender id: 1~99
* receiver id: 100~199
* broker id: 5000


### MSG_SEND_REQ/ACK
request 形式 (JSON)
``` json
{
    "msg_type": "MSG_SEND_REQ",
    "saddr": 1,
    "daddr": 100,
    "id": 12000,
    "payload": "hello"
}
```
response 形式 (JSON)
``` json
{
    "code": 200,
    "status": "OK",
    "msg_type": "MSG_SEND_ACK"
}
```
### MSG_RECV_REQ/ACK
request 形式 (JSON)
``` json
{
    "msg_type": "MSG_RECV_REQ",
    "saddr": 100,
    "daddr": 5000
}
```
response 形式 (JSON)
``` json
{
    "code": 200,
    "status": "OK",
    "msg_type": "MSG_RECV_ACK",
    "saddr": 1,
    "daddr": 100,
    "id": 12000,
    "payload": "hello"
}
```
### MSG_FREE_REQ/ACK
request 形式 (JSON)
``` json
{
    "msg_type": "MSG_FREE_REQ",
    "saddr": 100,
    "daddr": 5000,
    "id": 12000
}
```
response 形式 (JSON)
``` json
{
    "code": 200,
    "status": "OK",
    "msg_type": "MSG_FREE_ACK"
}
```
### MSG_PUSH_REQ/ACK
このメッセージタイプはサポートしない．

### MSG_HELO_REQ/ACK
request 形式 (JSON)
``` json
{
    "msg_type": "MSG_HELO_REQ",
    "saddr": 1,
    "daddr": 5000
}
```
response 形式 (JSON)
``` json
{
    "code": 200,
    "status": "OK",
    "msg_type": "MSG_HELO_ACK"
}
```
### MSG_STAT_REQ/ACK
request 形式 (JSON)
``` json
{
    "msg_type": "MSG_STAT_REQ",
    "saddr": 1,
    "daddr": 5000
}
```
response 形式 (JSON)
``` json
{
    "code": 200,
    "status": "OK",
    "msg_type": "MSG_STAT_ACK",
    "payload": {}
}
```
### MSG_GBYE_REQ/ACK
request 形式 (JSON)
``` json
{
    "msg_type": "MSG_GBYE_REQ",
    "saddr": 1,
    "daddr": 5000
}
```
response 形式 (JSON)
``` json
{
    "code": 200,
    "status": "OK",
    "msg_type": "MSG_GBYE_ACK"
}
```
