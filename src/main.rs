use std::io::{stdout, Write};
use std::str::from_utf8;
use json;

//fn update_news(hotlist: Array(Vec<JsonValue>)) {
//
//}

//struct Newsentry {
//    title: String,
//    points: u32,
//    url: String,
//    time: i32
//}


fn main() {
    //let mut newslist: [Newsentry; 30];

   let body = reqwest::blocking::get("https://hacker-news.firebaseio.com/v0/topstories.json?print=pretty").unwrap();

   let mut list: Vec<u32> = body.json().unwrap();

   for id in list.iter().take(30){
     let entry_text : String = reqwest::blocking::get(format!("https://hacker-news.firebaseio.com/v0/item/{}.json?print=pretty",id)).unwrap().text().unwrap();
     let entry_json_val = json::parse(&entry_text).unwrap();
     let entry_json_obj = match entry_json_val {
            json::JsonValue::Object(obj) => obj,
            _ => panic!("expected json object"),
         };

       //println!("{:#?}", entry_json_obj)
       println!("{}", entry_text)
     }
    
 }
