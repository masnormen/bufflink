use ipinfo::{IpInfo, IpInfoConfig};
use postgres::{Client, NoTls};
use serde_json::Value;
use std::{collections::HashMap, error::Error};
use util::UAResult;
use vercel_lambda::{error::VercelError, lambda, IntoResponse, Request, Response};
use woothee::parser::Parser;

#[allow(non_snake_case)]
fn handler(req: Request) -> Result<impl IntoResponse, VercelError> {
    let DATABASE_URL: String = dotenv::var("DATABASE_URL").unwrap();
    let IPINFO_TOKEN: String = dotenv::var("IPINFO_TOKEN").unwrap();

    /* Parse link from url */

    let query: HashMap<&str, &str> = req
        .uri()
        .query()
        .unwrap_or_default()
        .split('&')
        .map(|s| {
            let mut split = s.split('=');
            (
                split.next().unwrap_or_default(),
                split.next().unwrap_or_default(),
            )
        })
        .collect();

    let link = query.get("link").unwrap_or(&"");

    if link.is_empty() {
        return Ok(Response::builder()
            .status(200)
            .header("Cache-Control", "public, max-age=0, must-revalidate")
            .body("Hey, you found my link shortener! Visit my site at: https://nourman.id/ :D")
            .expect("Internal server error"));
    }

    /* Get link target from database */

    let mut client = Client::connect(&DATABASE_URL, NoTls).unwrap();

    let links = client
        .query("SELECT * FROM links WHERE link = $1", &[link])
        .unwrap_or_default();

    if links.is_empty() {
        return Ok(Response::builder()
            .status(404)
            .header("Cache-Control", "public, max-age=0, must-revalidate")
            .body("Link not found!")
            .expect("Internal server error"));
    }

    let target = links[0].get::<_, String>("target");

    /* Parse user agent info */

    let ua_header = match req.headers().get("user-agent") {
        Some(ua) => ua.to_str().unwrap(),
        None => "",
    };

    let parser = Parser::new();
    let browser_info = Value::Object(UAResult::from(parser.parse(ua_header).unwrap()).into());

    /* Parse referrer info (from ?link= parameter and from `Referer` header) */

    let referrer_link = query.get("ref").unwrap_or(&"").to_string();
    let referrer_site = match req.headers().get("referer") {
        Some(referer) => referer.to_str().unwrap().to_owned(),
        None => "".to_owned(),
    };

    /* Parse IP info */

    let ip = match req.headers().get("x-real-ip") {
        Some(ip) => ip.to_str().unwrap().to_owned(),
        None => "".to_owned(),
    };

    let ipinfo_client = IpInfo::new(IpInfoConfig {
        token: Some(IPINFO_TOKEN),
        ..Default::default()
    });

    let ip_info = if let Ok(mut ipinfo) = ipinfo_client {
        let ip_binding = ipinfo.lookup(&[&ip]).unwrap_or_default();
        let ip_map = ip_binding.get(&ip);
        match ip_map {
            Some(info) => serde_json::to_value(info).unwrap(),
            None => serde_json::to_value("").unwrap(),
        }
    } else {
        serde_json::to_value("").unwrap()
    };

    client
        .execute(
            "INSERT INTO links_view (link, browser_info, referrer_link, referrer_site, ip_info) VALUES ($1, $2, $3, $4, $5)",
            &[
                &link,
                &browser_info,
                &referrer_link,
                &referrer_site,
                &ip_info
            ],
        )
        .expect("Error when inserting log");

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
