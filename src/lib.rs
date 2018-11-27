extern crate chrono;
extern crate clap;
extern crate reqwest;
extern crate serde_derive;
extern crate serde_json;

use chrono::{DateTime, Utc, Local};
use reqwest::header::{qitem, Accept, Authorization, Bearer, Link, RelationType, UserAgent};
use serde_derive::Deserialize;
use std::{error, fmt, mem};
use std::time::{UNIX_EPOCH, Duration};

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
}

impl fmt::Display for Repository {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]({})", self.full_name, self.html_url)?;

        if let Some(ref description) = self.description {
            write!(f, " - {}", description)?;
        }

        Ok(())
    }
}

pub fn collect_stars(config: Config) -> Result<(), Box<dyn error::Error>> {
    let mut builder = ClientBuilder::new();

    if let Some(ref token) = config.token {
        builder.set_authorization_token(token.to_owned());
    }
    else {
        println!("Authentication Warning: This is an unauthenticated request with a limit of 60 requests per hour. Re-run this program using an auth token by adding `--token <auth-token>` for an increased quota of 5000 requests per hour.")
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
    let total_rate_limit = "X-RateLimit-Limit";
    let rate_limit_remaining = "X-RateLimit-Remaining";
    let rate_limit_reset_time = "X-RateLimit-Reset";   
    
    let mut remaining: i32 = 0; 
    let mut total: i32 = 0;

    for header in headers.iter() {
       if header.name() == total_rate_limit {
           total = header.value_string().parse::<i32>().unwrap();
       }
       if header.name() == rate_limit_remaining {
           remaining = header.value_string().parse::<i32>().unwrap();
       }
       if header.name() == rate_limit_reset_time {
            let reset_time = header.value_string();
            let timestamp = reset_time.parse::<u64>().unwrap();

            // Creates a new SystemTime from the specified number of whole seconds
            let d = UNIX_EPOCH + Duration::from_secs(timestamp);
            
            // Create DateTime from SystemTime
            let datetime = DateTime::<Local>::from(d);

            // Formats the combined date and time with the specified format string.
            let timestamp_str = datetime.format("%Y-%m-%d %I:%M").to_string();
            println!{"You have {} out of {} requests remaining. Your request limit will reset at {}", remaining, total, timestamp_str};
        }
    }
   
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
