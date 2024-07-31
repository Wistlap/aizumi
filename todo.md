# すべきこと


## network/api.rs の代わりコード
* /write の代わりになるコード
    * / に MsgType=Send，


## 今後のコードイメージ
* API: / (POST) のままでいい
* / に届いた Msg の MsgType に応じて raft::client_write, raft::clinet_read 的なコードを呼び出す (今の network/api.rs がやっていること)
* raft::client_write とかをされたときのコード trait RaftStateMachine を設定する (apply の時に Response を作成する処理を書いてやる)

## 問題
以下のコードが現在の状態を反映できていない
```rust
// lib.rs

// Start the actix-web server.
let server = HttpServer::new(move || {
    actix_web::App::new()
        .wrap(Logger::default())
        .wrap(Logger::new("%a %{User-Agent}i"))
        .wrap(middleware::Compress::default())
        .app_data(app_data.clone())
        .app_data(queue.clone())
        // raft internal RPC
        .service(raft::append)
        .service(raft::snapshot)
        .service(raft::vote)
        // admin API
        .service(management::init)
        .service(management::add_learner)
        .service(management::change_membership)
        .service(management::metrics)
        // application API
        // .service(api::write)
        // .service(api::read)
        // .service(api::consistent_read)
        .route("/", web::post().to(submit))
});
```

* 以下のコードの代わりとなるコードを書く必要がある．
* treat_msg の中で MsgType に応じて書く必要がある．
```rust
// network/api.rs

#[post("/write")]
pub async fn write(app: Data<App>, req: Json<Request>) -> actix_web::Result<impl Responder> {
    let response = app.raft.client_write(req.0).await;
    Ok(Json(response))
}

#[post("/read")]
pub async fn read(app: Data<App>, req: Json<String>) -> actix_web::Result<impl Responder> {
    let state_machine = app.state_machine_store.state_machine.read().await;
    let key = req.0;
    let value = state_machine.data.get(&key).cloned();

    let res: Result<String, Infallible> = Ok(value.unwrap_or_default());
    Ok(Json(res))
}

#[post("/consistent_read")]
pub async fn consistent_read(app: Data<App>, req: Json<String>) -> actix_web::Result<impl Responder> {
    let ret = app.raft.ensure_linearizable().await;

    match ret {
        Ok(_) => {
            let state_machine = app.state_machine_store.state_machine.read().await;
            let key = req.0;
            let value = state_machine.data.get(&key).cloned();

            let res: Result<String, RaftError<NodeId, CheckIsLeaderError<NodeId, BasicNode>>> =
                Ok(value.unwrap_or_default());
            Ok(Json(res))
        }
        Err(e) => Ok(Json(Err(e))),
    }
}
```

```rust
// store/mod.rs

#[tracing::instrument(level = "trace", skip(self, entries))]
async fn apply<I>(&mut self, entries: I) -> Result<Vec<Response>, StorageError<NodeId>>
where I: IntoIterator<Item = Entry<TypeConfig>> + Send {
    let mut res = Vec::new(); //No `with_capacity`; do not know `len` of iterator

    let mut sm = self.state_machine.write().await;

    for entry in entries {
        tracing::debug!(%entry.log_id, "replicate to sm");

        sm.last_applied_log = Some(entry.log_id);

        match entry.payload {
            EntryPayload::Blank => res.push(Response { value: None }),
            EntryPayload::Normal(ref req) => match req {
                Request::Set { key, value } => {
                    sm.data.insert(key.clone(), value.clone());
                    res.push(Response {
                        value: Some(value.clone()),
                    })
                }
            },
            EntryPayload::Membership(ref mem) => {
                sm.last_membership = StoredMembership::new(Some(entry.log_id), mem.clone());
                res.push(Response { value: None })
            }
        };
    }
    Ok(res)
}
```
