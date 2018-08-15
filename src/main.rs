#[macro_use]
extern crate serde_derive;
extern crate chrono;
extern crate reqwest;
extern crate serde_json;

use chrono::{DateTime, Utc};
use reqwest::header::{qitem, Accept, Authorization, Bearer, Link, RelationType, UserAgent};
use std::{env, fmt, process};

#[derive(Debug)]
struct Config {
    username: String,
    token: Option<String>,
}

impl Config {
    fn new(mut args: env::Args) -> Result<Config, &'static str> {
        args.next();

        let username = match args.next() {
            None => return Err("No username provided"),
            Some(arg) => arg,
        };

        let token = args.next();

        Ok(Config { username, token })
    }
}

#[derive(Debug)]
struct ClientBuilder {
    inner: reqwest::ClientBuilder,
    headers: reqwest::header::Headers,
}

impl ClientBuilder {
    fn new() -> ClientBuilder {
        let mut headers = reqwest::header::Headers::new();
        headers.set(Accept(vec![qitem(
            "application/vnd.github.v3.star+json".parse().unwrap(),
        )]));
        headers.set(UserAgent::new("supernova/0.1.0"));

        ClientBuilder {
            inner: reqwest::ClientBuilder::new(),
            headers,
        }
    }

    fn build(&mut self) -> reqwest::Result<reqwest::Client> {
        self.inner.build()
    }

    fn set_authorization_token(&mut self, token: String) -> &mut ClientBuilder {
        self.headers.set(Authorization(Bearer { token }));
        self
    }
}

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
    let config = Config::new(env::args()).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    let mut builder = ClientBuilder::new();

    if let Some(ref token) = config.token {
        builder.set_authorization_token(token.to_owned());
    }

    let client = builder.build()?;

    let mut stars: Vec<Star> = Vec::new();

    let mut next_link = Some(format!(
        "https://api.github.com/users/{}/starred",
        config.username
    ));

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

fn extract_link_next(headers: &reqwest::header::Headers) -> Option<String> {
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
