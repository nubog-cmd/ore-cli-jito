# Ore CLI

A command line interface for the Ore program using Jito Bundles.

# My Server
Joing my discord for support and a bunch of cool tools like jito pool bundling scripts, volume bots, maker bots, any other cool stuff for token launches :)

Server: https://discord.gg/rn84eaRv7Y

My discord: testicklez

My TG: @testicklez

# Building

To build the Ore CLI, you will need to have the Rust programming language installed. You can install Rust by following the instructions on the [Rust website](https://www.rust-lang.org/tools/install).

Once you have Rust installed, you can build the Ore CLI by running the following commands in order:

1. Install protoc for jito:
    `wget https://github.com/protocolbuffers/protobuf/releases/download/v26.1/protoc-26.1-linux-x86_64.zip`
    `sudo unzip -o protoc-26.1-linux-x86_64.zip -d /usr/local bin/protoc`
    `sudo unzip -o protoc-26.1-linux-x86_64.zip -d /usr/local 'include/*'`

2. Put your jito blockengine keypair inside of auth.json.  

3. Edit the keys.txt with all 25 private keys of your miners.

4. Edit the payer.json with the keypair that will pay all txn fees and jito tip.

5. Build with `cargo build --release`

6. Finally run: 
`./target/release/ore --rpc "" --be-url "https://frankfurt.mainnet.block-engine.jito.wtf" --feepayer ./payer.json --auth ./auth.json --jito-enable --jito-fee 500000 mine --threads 8`

# Happy mining :salute:
