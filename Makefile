.PHONY: replica stop-replica local

replica:
	dfx ping &>/dev/null || dfx start --clean --background

stop-replica:
	dfx stop

local: replica
	dfx deploy

clean: stop-replica
	cargo clean