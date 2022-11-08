### How to run this.

```bash
# terminal #1, start up rmq cluster
cd config
docker-compose up

# terminal #2, run the server
# running shared runtime for mock telemetry task
cargo run --bin shared
# running exclusive runtime for mock telemetry task
cargo run --bin exclusive

# terminal #3, run the client
# cargo run --bin client <rounds> <invokes-per-round> <interval-between-each-round>, e.g
cargo run --bin client 3 10000 1
```