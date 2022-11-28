//! # Docker tester
//! This library provides simple functions for starting and stopping containers using Docker
//!
//! ## Getting started
//! You must have Docker installed and started
//!
//! ```rust
//! use docker_tester::start_container;
//!
//! fn main() {
//!     let image = "postgres:latest"
//!     let port = "5432"
//!     let args = &[
//!         "-e",
//!         "POSTGRES_USER=postgres",
//!         "-e",
//!         "POSTGRES_PASSWORD=password"
//!     ];
//!     let container = start_container(image, port, args)
//!         .expect("Failed to start Postgres contaienr");    
//!     assert!(container.id);
//!     assert!(container.host);
//!     assert!(container.port);
//! }
//! ```
//!
//! ## db-tester
//!
//! ```rust
//! use docker_tester::TestPostgres;
//!
//! #[tokio::test]
//! async fn it_works() {
//!     let test_postgres = TestPostgres::new("./migrations").await.unwrap();
//!     let pool = test_postgres.get_pool().await;
//!
//!     // do something with the pool
//!
//!     // when test_postgres gets dropped, the database will be dropped on Docker
//! }
//! ```

mod db_tester;
pub use db_tester::TestPostgres;

use std::process::Command;
use std::{thread, time};

use serde::{Deserialize, Serialize};

/// Container tracks information about the docker container started for tests.
pub struct Container {
    pub id: String,
    pub host: String,
    pub port: u16,
}

/// Starts the specified container for running tests.
///
/// # Example
/// ```
/// let image = "postgres:14-alpine"
/// let port = "5432"
/// let args = &[
///    "-e",
///    "POSTGRES_USER=postgres",
///    "-e",
///    "POSTGRES_PASSWORD=password",
/// ];
/// let container = start_container(image, port, args).expect("Failed to start Postgres container");
/// assert!(container.id);
/// assert!(container.host);
/// assert!(container.port);
/// ```
pub fn start_container(image: &str, port: &str, args: &[&str]) -> Result<Container, anyhow::Error> {
    let output = Command::new("docker")
        .arg("run")
        .arg("-P")
        .arg("-d")
        .args(args)
        .arg(&image)
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(String::from_utf8(output.stderr)?));
    }
    let output = String::from_utf8(output.stdout)?;

    let id = &output[..12];
    let ns = extract_ip_and_port(id, port)?;
    let host = format!("{}:{}", ns.host_ip, ns.host_port);

    for i in 1..=10 {
        let output = Command::new("docker")
            .arg("inspect")
            .arg("-f")
            .arg("{{.State.Status}}")
            .arg(&id)
            .output()?;
        let output = String::from_utf8(output.stdout)?;
        let output = output.trim();
        if output == "running" {
            println!(
                r#"
Docker Started
Image:       {image}
ContainerID: {id}
Host:        {host}
                "#
            );
            break;
        } else {
            if i == 10 {
                return Err(anyhow::anyhow!("cannot start the image[{image}] container"));
            }
            println!("Container[{id}] state {output}, Watting for start");
            thread::sleep(time::Duration::from_secs(i));
        }
    }

    Ok(Container {
        id: id.to_string(),
        host: ns.host_ip,
        port: ns.host_port.parse::<u16>().unwrap(),
    })
}

/// Stops and removes the specified container.
///
/// # Example
///
/// ```
/// let container_id = "dfd60e4ef0c0";
/// stop_container(container_id).expect("Failed to stop the container");
/// ```
pub fn stop_container(id: String) -> Result<(), anyhow::Error> {
    let output = Command::new("docker").arg("stop").arg(&id).output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(String::from_utf8(output.stderr)?));
    }

    let output = Command::new("docker")
        .arg("rm")
        .arg(&id)
        .arg("-v")
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(String::from_utf8(output.stderr)?));
    }
    Ok(())
}

fn extract_ip_and_port(id: &str, port: &str) -> Result<NetworkSettings, anyhow::Error> {
    let tmpl = format!(
        r#"'[{{{{range $k,$v := (index .NetworkSettings.Ports "{port}/tcp")}}}}{{{{json $v}}}}{{{{end}}}}]'"#
    );
    let output = Command::new("docker")
        .arg("inspect")
        .arg("-f")
        .arg(tmpl)
        .arg(&id)
        .output()?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(String::from_utf8(output.stderr)?));
    }

    let json_string = String::from_utf8(output.stdout)?;
    let datas: Vec<NetworkSettings> = serde_json::from_str(&json_string.trim().trim_matches('\''))?;
    assert!(
        datas.len() >= 1,
        "The container[{id}] cannnot find NetworkSettings.Ports"
    );
    let mut network_settings = NetworkSettings::default();
    if let Some(ns) = datas.first() {
        network_settings.host_ip = ns.host_ip.clone();
        network_settings.host_port = ns.host_port.clone();
    }
    Ok(network_settings)
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct NetworkSettings {
    #[serde(alias = "HostIp")]
    host_ip: String,

    #[serde(alias = "HostPort")]
    host_port: String,
}

#[test]
fn start_and_stop_container() {
    let image = "docker/getting-started";
    let port = "80";
    let args = &[];
    let container = start_container(image, port, args).unwrap();
    stop_container(container.id).unwrap();
}
