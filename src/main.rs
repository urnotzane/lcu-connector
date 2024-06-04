use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use teemo::Teemo;
use tokio::sync::RwLock;
use tokio::task;

struct AppState {
    lcu_activated: RwLock<bool>,
}
#[derive(Deserialize, Serialize)]
struct TaskRequest {
    param: String,
}

async fn greet() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

async fn check_lcu(state: Arc<AppState>) {
    loop {
        let mut teemo = Teemo::new();

        let mut activated = state.lcu_activated.write().await;
        *activated = teemo.init_lcu_config();

        let mut lcu_state = "offline";

        if *activated {
            lcu_state = "online";
        }
        println!(
            "LCU is {}.Program will check LCU every 5 seconds.",
            lcu_state
        );

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn teemo_req(
    req: HttpRequest,
    state: web::Data<Arc<AppState>>,
    query: web::Query<std::collections::HashMap<String, String>>,
    body:  web::Json<TaskRequest>,
) -> impl Responder {
    if !*state.lcu_activated.read().await {
        return HttpResponse::BadGateway().body("Please start the LOL client first.");
    }
    let lcu_url = query.get("url").unwrap_or(&"default".to_string()).clone();
    // 获取请求方法
    let method = req.method().as_str();
    let mut teemo = Teemo::new();
    teemo.start();
    // 将 web::Json<TaskRequest> 转换为 serde_json::Value
    let body_value: Value = serde_json::to_value(&body).unwrap();
     // 将 serde_json::Value 转换为 HashMap<String, Value>
     let body_map: HashMap<String, Value> = match body_value {
        Value::Object(map) => map.into_iter().collect(),
        _ => HashMap::new(),
    };
    let res = teemo.request(&method, &lcu_url, Some(body_map)).await;
    HttpResponse::Ok().body(res.to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = Arc::new(AppState {
        lcu_activated: RwLock::new(false),
    });

    let check_state = state.clone();

    // 创建并启动检查服务连接状态的线程
    task::spawn(async move {
        check_lcu(check_state).await;
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .route("/", web::get().to(greet))
            .route("/lcu_api", web::to(teemo_req))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
