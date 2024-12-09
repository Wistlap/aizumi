use futures::future::join_all;
use reqwest::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let works: Vec<_> = (0..1000).map(|i| send_request(i)).collect();
    let _ret = join_all(works).await;
    Ok(())
}

async fn send_request(i: i32) {
    let json_data =
        r#"{"msg_type": "MSG_SEND_REQ","saddr": 100,"daddr": 5000,"id": 1,"payload": "hello"}"#;
    let client = reqwest::Client::new();
    let _res = client
        .post("http://localhost:8080/")
        .header("Content-Type", "application/json")
        .body(json_data)
        .send()
        .await
        .unwrap();

    println!("{i}");
}
