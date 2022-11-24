# docker-tester

This library provides simple functions for starting and stopping containers using Docker.

## Getting started

You must have Docker installed and started

```rust
use docker_tester::start_container;

fn main() {
    let image = "postgres:latest"
    let port = "5432"
    let args = &[
        "-e",
        "POSTGRES_USER=postgres",
        "-e",
        "POSTGRES_PASSWORD=password"
    ];
    let container = start_container(image, port, args)
        .expect("Failed to start Postgres contaienr");
}
```
