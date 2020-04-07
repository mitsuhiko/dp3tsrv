server:
	@RUST_LOG=debug cargo run
.PHONY: server

server-reload:
	@RUST_LOG=debug systemfd --no-pid -s http::5000 -- cargo watch -x run
.PHONY: server-reload
