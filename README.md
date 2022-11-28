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

    assert!(container.id);
    assert!(container.host);
    assert!(container.port);
}
```

## db-tester

```rust
use docker_tester::TestPostgres;

#[tokio::test]
async fn it_works() {
    let test_postgres = TestPostgres::new("./migrations").await.unwrap();
    let pool = test_postgres.get_pool().await;

    // do something with the pool

    // when test_postgres gets dropped, the database will be dropped on Docker
}
```

## License

This project is distributed under the terms of MIT.

See [LICENSE](./LICENSE) for details.

Copyright 2022 startdusk
