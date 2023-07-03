# Stacks Signer API

This is a CLI application for the Stacks Signer API. It provides an API server for interacting with a Signer. It can run an API server with or without simulated data, generate API documentation, and run a Swagger UI server.

## Requirements

- Rust
- SQLite


## Getting Started

To get started, first, make sure you have Rust installed. You can follow the Rust installation instructions [here](https://www.rust-lang.org/tools/install).

Next, clone this project and navigate to its root directory in your command prompt or terminal.

1. Clone the repository:

```bash
git clone https://github.com/Trust-Machines/stacks-sbtc.git
```

2. Navigate to the project folder:

```bash
cd stacks-sbtc/stacks-signer-api
```

3. Build the CLI:

```bash
cargo build --release
```

4. Navigate to the output folder:

```bash
cd target/release
```

## Usage

### Run

To run the API server, run the following command

```shell
./stacks-signer-api run
```

#### Arguments
- `--address` - Address to run the API server on (Default: `0.0.0.0`)
- `--port` - Port to run the API server on (Default: `3030`)


### Simulator

To run the API server with simulated data, run the following command

```shell
./stacks-signer-api simulator
```

#### Arguments
- `--address` - Address to run the API server on (Default: `0.0.0.0`)
- `--port` - Port to run the API server on (Default: `3030`)

### Swagger

To run a local instance of Swagger UI for this API, execute the following command:

```shell
./stacks-signer-api swagger
```

#### Arguments
- `--address` - Address to run the API server on (Default: `0.0.0.0`)
- `--port` - Port to run the API server on (Default: `3030`)

By default, the Swagger UI server will be accessible at `http://0.0.0.0:3030/swagger-ui/`.

### Docs

To generate OpenAPI json documentation for the API, run the following command:

```shell
./stacks-signer-api docs
```

#### Args
- `--output` - Output file to save the API documentation to. If not provided, it prints to stdout.



## Endpoints

Run the Swagger CLI option to obtain a detailed breakdown of the API endpoints, requests, and responses.


## Error Handling

The Signer API uses a custom error type `Error`, which wraps SQLx and parse errors. The error type is defined in the root of the project and implements the `warp::reject::Reject` trait to enable proper handling in Warp filters.

## Database

The Signer API uses SQLite as the backend database, and the `sqlx` crate is utilized for handling database connections and queries.

Database operations are organized in separate modules according to their purpose (e.g., `keys`, `signers`, `transaction`, `vote`). Each module contains functions for interacting with the database, such as retrieving records, inserting records, and deleting records.

### Database Configuration

To use a file-based SQLite database, set the `DATABASE_URL` environment variable with the file path:

```shell
export DATABASE_URL="sqlite://path/to/your/database.sqlite3"
```

If the `DATABASE_URL` environment variable is not set, the API will use an in-memory SQLite database by default. Note that using an in-memory database is suitable for testing purposes but not recommended for production use, as the data will not be persisted after terminating the application.

To run the server with the database specified in the `DATABASE_URL` environment variable (or an in-memory database if the variable is not set), use the following command:

```shell
./stacks-signer-api run
```

Alternatively, run the following command:

```shell
DATABASE_URL="sqlite://path/to/your/database.sqlite3" ./stacks-signer-api run
```

## Dev/Compilation setup

To make use of `sqlx` and verify the sql queries on your own, you should follow the following steps:

1. remove `sqlx-data.json`
2. install `sqlx-cli` version `0.5.13`. So `cargo install sqlx-cli --version=0.5.13`
3. make sure you have sqlite installed
4. create a `.env` file in the `stacks-signer-api` root folder with the env variable `DATABASE_URL`
5. The url for sqlite is in the format `DATABASE_URL=sqlite://$(pwd)/stacks-signer-api/dev-signer-api.sqlite`
6. generate the test db using `sqlx database create`
7. run the `init` migration `sqlx migrate run`
8. prepare the `offline` static check cache `cargo sqlx prepare -- --lib`

## License

This project is licensed under [GPLv3](../LICENSE).