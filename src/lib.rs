extern crate chrono;
extern crate clap;
extern crate reqwest;
extern crate serde_derive;
extern crate serde_json;

use chrono::{DateTime, Utc};
use reqwest::header::{qitem, Accept, Authorization, Bearer, Link, RelationType, UserAgent};
use reqwest::StatusCode;
use serde_derive::Deserialize;
use std::time::{UNIX_EPOCH, SystemTime, Duration};
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
    let mut print_auth_warning = false; 

    if let Some(ref token) = config.token {
        builder.set_authorization_token(token.to_owned());
    }
    else {
        print_auth_warning = true; 
    }

    let client = builder.build()?;

    let mut stars: Vec<Star> = Vec::new();

    let mut next_link = config.url();
    
    let mut remaining: i32 = 0; 
    let mut total: i32 = 0;
    let mut mins = 0;

    while next_link.is_some() {
        if let Some(link) = next_link {
            let mut res = client.get(&link).send()?;

            for header in res.headers().iter() {
                match header.name() {
                    "X-RateLimit-Limit" => {
                        total = header.value_string().parse::<i32>()?;
                    },
                    "X-RateLimit-Remaining" => { 
                        remaining = header.value_string().parse::<i32>()?;
                    },
                    "X-RateLimit-Reset" => {
                        let seconds = header.value_string().parse::<u64>()?;

                        // Creates a new SystemTime from the specified number of whole seconds
                        let reset_time = UNIX_EPOCH + Duration::from_secs(seconds);
                        mins = reset_time.duration_since(SystemTime::now())?.as_secs()/60;
                    },
                    _ =>(),
                }
            }
            
            match res.status() {
                StatusCode::Forbidden => {
                    return Err(format!("Uh-oh! You have {} out of {} requests remaining. Your request limit will reset in {} minutes.", remaining, total, mins).into());
                },
                _ => (),
            }

            next_link = extract_link_next(res.headers());
        
            let mut s: Vec<Star> = res.json()?;
            stars.append(&mut s);
        }
    }

    for star in stars.iter() {
        println!("{}", star);
    }
    println!("Collected {} stars", stars.len());

    if print_auth_warning {
        eprintln!("\nRequest completed without authentication, re-run and provide token using `--token <auth-token>` to increase your requests from 60 to 5000 requests per hour.");
    }

    match remaining {
        10 | 0...5 => eprintln!("Warning: You have {} out of {} requests remaining. Your request limit will reset in {} minutes.", remaining, total, mins),
        _ => (),
    }

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
