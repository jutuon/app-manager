# app-manager

Manage app VPS instance.

## Running

RUST_LOG=debug cargo run

## Building on MacOS

TODO

## Building on Ubuntu

sudo apt install build-essential libssl-dev pkg-config

# Local development servers

Multipass logs
less /Library/Logs/Multipass/multipassd.log

You can get multipass default SSH public key by creating default VM and checking
.ssh dir.

Don't try to create user named admin. It will not work.

## Update manager API bindings

1. Install node version manager (nvm) <https://github.com/nvm-sh/nvm>
2. Install latest node LTS with nvm. For example `nvm install 18`
3. Install openapi-generator from npm.
   `npm install @openapitools/openapi-generator-cli -g`
4. Start app backend in debug mode.
5. Generate bindings
```
openapi-generator-cli generate -i http://localhost:4000/api-doc/app_api.json -g rust -o manager_api_client --package-name manager_api_client
```
