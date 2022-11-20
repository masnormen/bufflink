use postgres::{Client, NoTls};
use serde_json::Value;
use std::{collections::HashMap, error::Error};
use util::UAResult;
use vercel_lambda::{error::VercelError, lambda, IntoResponse, Request, Response};
use woothee::parser::Parser;

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
            .header("Cache-Control", "public, max-age=0, must-revalidate")
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
            .header("Cache-Control", "public, max-age=0, must-revalidate")
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
    let ip = match req.headers().get("x-real-ip") {
        Some(ip) => ip.to_str().unwrap().to_owned(),
        None => "".to_owned(),
    };

    client
        .execute(
            "INSERT INTO links_view (link, browser_info, referrer_link, referrer_site, ip) VALUES ($1, $2, $3, $4, $5)",
            &[
                &link,
                &browser_info,
                &referrer_link,
                &referrer_site,
                &ip
            ],
        )
        .unwrap();

    let response = Response::builder()
        .status(308)
        .header("Location", target)
        .header("Cache-Control", "public, max-age=0, must-revalidate")
        .body("Redirecting...")
        .expect("Internal server error");

    Ok(response)
}

fn main() -> Result<(), Box<dyn Error>> {
    Ok(lambda!(handler))
}
