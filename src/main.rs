use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use json;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::task::spawn_blocking;

const NEWSITEMS: usize = 5;
const HOTSCORE: u32 = 250;

#[derive(Debug)]
struct NewsItem {
    id: u32,
    title: String,
    url: Option<String>,
    score: u32,
    seen: Instant,
}

async fn handle(
    _req: Request<Body>,
    news: Arc<Mutex<Vec<NewsItem>>>,
) -> Result<Response<Body>, Infallible> {

    let mut newsitems: String = "".to_owned();

    for newsitem in &*news.lock().unwrap() {

        let mut newsclass: String = "".to_string();

        if newsitem.score >= HOTSCORE {
            newsclass.push_str("hot");
        }

        if let Some(url) = &newsitem.url {
            newsitems.push_str(&format!("<li class='{}'><a href='{}'>{}</a></li>", newsclass, url, newsitem.title));
        }
        else {
            newsitems.push_str(&format!("<li class='{}'>{}</li>", newsclass, newsitem.title));
        }

        println!("item {:#?}", newsitem);

    }

    Ok(Response::new(Body::from(format!(
        "<body>
                <style>
                    .hot {{ background-color: yellow;}}
                </style>
                <h1>No old hacker news</h1>
                <ol>{}</ol>
        </body>",
        newsitems
    ))))
}

#[tokio::main]
async fn main() {
    let news = Arc::new(Mutex::new(
        spawn_blocking(|| update_news(&[])).await.unwrap(),
    ));
    let news2 = news.clone();

    println!("Serving on 127.0.0.1:8000");

    // Construct our SocketAddr to listen on...
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    // And a MakeService to handle each connection...
    let make_service = make_service_fn(move |_conn| {
        let news2 = news.clone();
        async move {
            let news3 = news2.clone();
            Ok::<_, Infallible>(service_fn(move |req| handle(req, news3.clone())))
        }
    });

    let _join_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5 * 60)).await;
            let news3 = news2.clone();
            let new_news = spawn_blocking(move || {
                let news_guard = news3.lock().unwrap();
                update_news(&*news_guard)
            })
            .await
            .unwrap();
            let mut news_guard = news2.lock().unwrap();
            *news_guard = new_news;
        }
    });

    // Then bind and serve...
    let server = Server::bind(&addr).serve(make_service);

    // And run forever...
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

fn update_news(old_news: &[NewsItem]) -> Vec<NewsItem> {
    println!("Populating...");

    let body = reqwest::blocking::get(
        "https://hacker-news.firebaseio.com/v0/topstories.json?print=pretty",
    )
    .unwrap();

    let list: Vec<u32> = body.json().unwrap();

    let mut newslist = Vec::with_capacity(NEWSITEMS);

    for id in list.iter().take(NEWSITEMS) {
        let entry_text: String = reqwest::blocking::get(format!(
            "https://hacker-news.firebaseio.com/v0/item/{}.json?print=pretty",
            id
        ))
        .unwrap()
        .text()
        .unwrap();
        let entry_json_val = json::parse(&entry_text).unwrap();
        let entry_json_obj = match entry_json_val {
            json::JsonValue::Object(obj) => obj,
            _ => panic!("expected json object"),
        };

        let mut item = NewsItem {
            id: entry_json_obj["id"].as_u32().unwrap(),
            title: entry_json_obj["title"].as_str().unwrap().to_owned(),
            url: entry_json_obj
                .get("url")
                .map(|u| u.as_str().unwrap().to_owned()),
            score: entry_json_obj["score"].as_u32().unwrap(),
            seen: Instant::now(),
        };

        for x in old_news {
            if x.id != item.id {
                continue;
            }
            item.seen = x.seen;
        }

        newslist.push(item);
    }

    newslist
}
