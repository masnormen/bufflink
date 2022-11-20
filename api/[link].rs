use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{borrow::Cow, collections::HashMap, error::Error};
use vercel_lambda::{error::VercelError, lambda, IntoResponse, Request, Response};
use woothee::parser::{Parser, WootheeResult};

#[derive(Serialize, Deserialize, Debug)]
struct UAResult<'a> {
    pub name: &'a str,
    pub category: &'a str,
    pub os: &'a str,
    pub os_version: Cow<'a, str>,
    pub browser_type: &'a str,
    pub version: &'a str,
    pub vendor: &'a str,
}

impl<'a> From<WootheeResult<'a>> for UAResult<'a> {
    fn from(def: WootheeResult<'a>) -> UAResult<'a> {
        UAResult {
            name: def.name,
            category: def.category,
            os: def.os,
            os_version: def.os_version,
            browser_type: def.browser_type,
            version: def.version,
            vendor: def.vendor,
        }
    }
}

impl Into<Map<String, Value>> for UAResult<'_> {
    fn into(self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("name".to_owned(), Value::String(self.name.to_owned()));
        map.insert(
            "category".to_owned(),
            Value::String(self.category.to_owned()),
        );
        map.insert("os".to_owned(), Value::String(self.os.to_owned()));
        map.insert(
            "os_version".to_owned(),
            Value::String(self.os_version.to_string()),
        );
        map.insert(
            "browser_type".to_owned(),
            Value::String(self.browser_type.to_owned()),
        );
        map.insert("version".to_owned(), Value::String(self.version.to_owned()));
        map.insert("vendor".to_owned(), Value::String(self.vendor.to_owned()));
        map
    }
}

fn handler(req: Request) -> Result<impl IntoResponse, VercelError> {
    /* Parse link from url */

    let query: HashMap<&str, &str> = req
        .uri()
        .query()
        .unwrap()
        .split('&')
        .map(|s| {
            let mut split = s.split('=');
            (split.next().unwrap(), split.next().unwrap())
        })
        .collect();

    if !query.contains_key("link") {
        return Ok(Response::builder()
            .status(200)
            .body("Hey, you found my link shortener!")
            .unwrap());
    }

    let link = query.get("link").unwrap();

    /* Get link target from database */

    let mut client = Client::connect(&dotenv::var("DATABASE_URL").unwrap(), NoTls).unwrap();

    let links = client
        .query("SELECT * FROM links WHERE link = $1", &[link])
        .unwrap_or_default();

    if links.is_empty() {
        return Ok(Response::builder()
            .status(404)
            .body("Link not found!")
            .unwrap());
    }

    let target = links[0].get::<_, String>("target");

    /* Parse user agent info */

    let parser = Parser::new();
    let browser_info = Value::Object(
        UAResult::from(
            parser
                .parse(req.headers().get("user-agent").unwrap().to_str().unwrap())
                .unwrap(),
        )
        .into(),
    );

    /* Parse referrer info (from ?link= parameter and from `Referer` header) */

    let referrer_link = query.get("ref").unwrap_or(&"").to_string();
    let referrer_site = match req.headers().get("referer") {
        Some(referer) => referer.to_str().unwrap().to_owned(),
        None => "".to_owned(),
    };

    client
        .execute(
            "INSERT INTO links_view (link, browser_info, referrer_link, referrer_site) VALUES ($1, $2, $3, $4)",
            &[
                &link,
                &browser_info,
                &referrer_link,
                &referrer_site,
            ],
        )
        .unwrap();

    let response = Response::builder()
        .status(308)
        .header("Location", target)
        .body("Redirecting...")
        .expect("Internal server error");

    Ok(response)
}

fn main() -> Result<(), Box<dyn Error>> {
    Ok(lambda!(handler))
}
