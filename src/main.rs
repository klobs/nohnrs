use std::io::{stdout, Write};
use std::str::from_utf8;
use json;
use std::time::Instant;

const NEWSITEMS: usize = 30;

#[derive(Debug)]
struct NewsItem {
    id: u32,
    title: String,
    url: String,
    score: u32,
    seen: Instant,
}

fn main() {
  let mut news = Vec::new();
  
  loop {
    news = update_news(&news[..]);
    std::thread::sleep(std::time::Duration::from_secs(5));
    println!("{:?}", news);
  }
}

fn update_news(old_news : &[NewsItem]) -> Vec<NewsItem> {
  let body = reqwest::blocking::get("https://hacker-news.firebaseio.com/v0/topstories.json?print=pretty").unwrap();

  let mut list: Vec<u32> = body.json().unwrap();
    
    let mut newslist = Vec::with_capacity(NEWSITEMS);
    
    for id in list.iter().take(NEWSITEMS){
      let entry_text : String = reqwest::blocking::get(format!("https://hacker-news.firebaseio.com/v0/item/{}.json?print=pretty",id)).unwrap().text().unwrap();
      let entry_json_val = json::parse(&entry_text).unwrap();
      let entry_json_obj = match entry_json_val {
        json::JsonValue::Object(obj) => obj,
        _ => panic!("expected json object"),
      };
      let mut item = NewsItem {
        id: entry_json_obj["id"].as_u32().unwrap(),
        title: entry_json_obj["title"].as_str().unwrap().to_owned(),
        url: entry_json_obj["url"].as_str().unwrap().to_owned(),
        score: entry_json_obj["score"].as_u32().unwrap(),
        seen: Instant::now()
      };
      
      for x in old_news {
        if x.id != item.id { continue; }
        item.seen = x.seen;
      }
    
      newslist.push(item);
    }
  
  newslist
}
