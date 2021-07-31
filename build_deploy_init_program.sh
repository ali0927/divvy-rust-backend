cargo build-bpf && solana program deploy -u https://api.devnet.solana.com --upgrade-authority ../divvy.json target/deploy/divvyexchange.so
#  && ts-node init_program.ts