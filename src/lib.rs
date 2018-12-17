extern crate chrono;
extern crate clap;
extern crate reqwest;
extern crate serde_derive;
extern crate serde_json;

use chrono::{DateTime, Utc};
use reqwest::header::{qitem, Accept, Authorization, Bearer, Link, RelationType, UserAgent};
use serde_derive::Deserialize;
use std::{error, fmt, mem};

#[derive(Debug)]
pub struct Config {
    username: String,
    token: Option<String>,
}

impl Config {
    fn url(self) -> Option<String> {
        Some(format!(
            "https://api.github.com/users/{}/starred",
            self.username
        ))
    }
}

impl<'a> From<clap::ArgMatches<'a>> for Config {
    fn from(matches: clap::ArgMatches) -> Self {
        Config {
            username: matches.value_of("USERNAME").unwrap().to_owned(),
            token: matches.value_of("TOKEN").map(String::from),
        }
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
        let headers = mem::replace(&mut self.headers, reqwest::header::Headers::new());
        self.inner.default_headers(headers).build()
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
    language: Option<String>, 
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]({})", self.full_name, self.html_url)?;

        if let Some(ref description) = self.description {
            write!(f, " - {}", description)?;
        }

        if let Some(ref language) = self.language {
            write!(f, "Main Language: {}", language)?;
        }

        Ok(())
    }
}

pub fn collect_stars(config: Config) -> Result<(), Box<dyn error::Error>> {
    let mut builder = ClientBuilder::new();

    if let Some(ref token) = config.token {
        builder.set_authorization_token(token.to_owned());
    }

    let client = builder.build()?;

    let mut stars: Vec<Star> = Vec::new();

    let mut next_link = config.url();

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
    println!("Collected {} stars", stars.len());

    Ok(())
}

fn extract_link_next(headers: &reqwest::header::Headers) -> Option<String> {
    let link_headers = headers.get::<Link>();

    match link_headers {
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
    }
}
