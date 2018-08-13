#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate reqwest;
extern crate serde_json;

use chrono::{DateTime, Utc};
use reqwest::header::{
    qitem, Accept, Authorization, Bearer, Headers, Link, RelationType, UserAgent,
};
use std::{env, fmt};

#[derive(Debug, Deserialize)]
struct Star {
    starred_at: DateTime<Utc>,
    repo: Repository,
}

impl fmt::Display for Star {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.repo)
    }
}

#[derive(Debug, Deserialize)]
struct Repository {
    id: i32,
    html_url: String,
    full_name: String,
    description: Option<String>,
    stargazers_count: i32,
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]({})", self.full_name, self.html_url)?;

        if let Some(ref description) = self.description {
            write!(f, " - {}", description)?;
        }

        return Ok(());
    }
}

fn main() -> Result<(), reqwest::Error> {
    let args: Vec<String> = env::args().collect();

    let client = build_client(args[2].to_owned())?;

    let mut stars: Vec<Star> = Vec::new();

    let mut next_link =
        Some(format!("https://api.github.com/users/{}/starred", &args[1]).to_owned());

    while next_link.is_some() {
        if let Some(link) = next_link {
            let mut res = client.get(&link).send()?;
            next_link = extract_link_next(res.headers());

            let mut s: Vec<Star> = res.json()?;
            stars.append(&mut s);
        }
    }

    for star in stars.iter() {
        println!("{}", star);
    }

    println!("collected {} stars", stars.len());

    Ok(())
}

fn build_client(token: String) -> reqwest::Result<reqwest::Client> {
    let mut headers = Headers::new();
    headers.set(Accept(vec![qitem(
        "application/vnd.github.v3.star+json".parse().unwrap(),
    )]));
    headers.set(UserAgent::new("supernova/0.1.0"));
    headers.set(Authorization(Bearer { token: token }));

    return reqwest::Client::builder().default_headers(headers).build();
}

fn extract_link_next(headers: &Headers) -> Option<String> {
    let link_headers = headers.get::<Link>();

    return match link_headers {
        None => None,
        Some(links) => links
            .values()
            .iter()
            .find(|&val| {
                val.rel().map_or(false, |rel| {
                    rel.first()
                        .map_or(false, |rel_type| rel_type == &RelationType::Next)
                })
            })
            .and_then(|link_value| Some(link_value.link().to_owned())),
    };
}
