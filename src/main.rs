use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Request, Response, Server, StatusCode};
use regex::Regex;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tokio::task::spawn_blocking;

const NEWSITEMS: usize = 30;
const HOTSCORE: u32 = 250;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

#[derive(Debug)]
struct NewsItem {
    id: u32,
    title: String,
    url: Option<String>,
    score: u32,
    seen: Duration,
}

fn get_classes(news_item: &NewsItem, seen: &Option<Duration>) -> String {
    let mut class_string: String = "".to_owned();
    if news_item.score >= HOTSCORE {
        class_string.push_str("hot");
    }
    if let Some(s) = seen {
        if !class_string.is_empty() {
            class_string.push(' ');
        }

        if news_item.seen.as_secs() < s.as_secs() {
            class_string.push_str("old");
        } else {
            class_string.push_str("new");
        }
    }
    class_string
}

fn get_seen_from_cookies(req: &Request<Body>) -> Option<Duration> {
    if req.headers().contains_key(header::COOKIE) {
        let re = Regex::new(r".*visit=(\d+).*").unwrap();
        let caps = re.captures(req.headers().get(header::COOKIE).unwrap().to_str().unwrap());
        if let Some(c) = caps {
            let last_visit = c.get(1).unwrap().as_str();
            // here, i would not like to use unwrap but use Result,
            // but i didnt manage it, as the Generic Error is defined above and
            // compiler complains
            let last_visit2: u64 = FromStr::from_str(last_visit).unwrap();
            println!("Last visit: {}", last_visit2);
            return Some(Duration::new(last_visit2, 0));
        }
        return None;
    }
    None
}

async fn handle(req: Request<Body>, news: Arc<Mutex<Vec<NewsItem>>>) -> Result<Response<Body>> {
    let mut newsitems: String = "".to_owned();

    let seen = get_seen_from_cookies(&req);

    for newsitem in &*news.lock().unwrap() {
        let newsclass: String = get_classes(newsitem, &seen);

        if let Some(url) = &newsitem.url {
            newsitems.push_str(&format!(
                "<li class='{}'><a href='{}'>{}</a><br>({} Points)</li>",
                newsclass, url, newsitem.title, newsitem.score
            ));
        } else {
            newsitems.push_str(&format!("<li class='{}'><a href='https://news.ycombinator.com/item?id={}'>{}</a><br>({} Points)</li>", newsclass, newsitem.id, newsitem.title, newsitem.score));
        }

        //println!("item {:#?}", newsitem);
    }

    let timestamp = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(
            header::SET_COOKIE,
            format!(
                "visit={}; SameSite=Strict; Max-Age=86400;",
                timestamp.unwrap().as_secs()
            ),
        )
        .header(header::CONTENT_TYPE, "text/html")
        .body(Body::from(format!(
            "<!doctype html>
                         <html><meta charset=\"utf-8\">
                                <style>
                                    li:nth-child(even) {{ background-color: #F0FFF0;}}
                                    .old {{ opacity: 0.33; }}
                                    .hot {{ background-color: yellow; opacity: 1;}}
                                </style>
                                <h1>No old hacker news</h1>
                                <ol>{}</ol>
                        </html>",
            newsitems
        )))?;

    Ok(response)
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
            Ok::<_, GenericError>(service_fn(move |req| handle(req, news3.clone())))
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
            seen: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
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
