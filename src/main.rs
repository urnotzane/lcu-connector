use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use teemo::Teemo;
use tokio::sync::{oneshot, RwLock};
use tokio::task;
use std::sync::Arc;
use std::collections::VecDeque;

struct AppState {
    task_queue: RwLock<VecDeque<(String, oneshot::Sender<String>)>>,
    lcu_activated: RwLock<bool>,
}

async fn greet() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

async fn process_task(param: String) -> String {
    let mut teemo = Teemo::new();
    teemo.init_lcu_config();
    format!("Processed: {}--{}", teemo.app_token, param)
}

async fn lcu_thread(state: Arc<AppState>) {
    loop {
        let task_opt = {
            let mut queue = state.task_queue.write().await;
            queue.pop_front()
        };
        
        if let Some((param, sender)) = task_opt {
            let result = process_task(param).await;
            let _ = sender.send(result);
        } else {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }
}

async fn check_lcu(state: Arc<AppState>) {
    loop {
        let mut teemo = Teemo::new();

        let mut activated = state.lcu_activated.write().await;
        *activated = teemo.init_lcu_config();

        println!("Program will check LCU every 5 seconds...");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

async fn handle_request(state: web::Data<Arc<AppState>>, info: web::Query<std::collections::HashMap<String, String>>) -> impl Responder {
    let param = info.get("param").unwrap_or(&"default".to_string()).clone();

    let (tx, rx) = oneshot::channel();

    {
        let mut queue = state.task_queue.write().await;
        queue.push_back((param.clone(), tx));
    }

    let result = rx.await.unwrap_or("Task failed".to_string());

    HttpResponse::Ok().body(result)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let state = Arc::new(AppState {
        task_queue: RwLock::new(VecDeque::new()),
        lcu_activated: RwLock::new(false),
    });

    let lcu_state = state.clone();
    let check_state = state.clone();

    // 创建并启动lcu线程
    task::spawn(async move {
        lcu_thread(lcu_state).await;
    });

    // 创建并启动检查服务连接状态的线程
    task::spawn(async move {
        check_lcu(check_state).await;
    });

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .route("/", web::get().to(greet))
            .route("/task", web::to(handle_request))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
