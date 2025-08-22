use qoranet::{
    transaction::{Transaction, TransactionData},
    fee_oracle::{GlobalFeeOracle, FeePriority, TransactionType},
    storage::BlockchainStorage,
    Address, Balance, LPToken, Result, QoraNetError,
};
use clap::{Arg, Command, ArgMatches, SubCommand};
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;
use std::path::PathBuf;
use std::fs;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("qoranet-cli")
        .version(qoranet::VERSION)
        .about("QoraNet Command Line Interface")
        .subcommand(
            Command::new("wallet")
                .about("Wallet operations")
                .subcommand(
                    Command::new("generate")
                        .about("Generate a new wallet keypair")
                        .arg(
                            Arg::new("output")
                                .short('o')
                                .long("output")
                                .help("Output file for the keypair")
                                .default_value("wallet.json")
                        )
                )
                .subcommand(
                    Command::new("balance")
                        .about("Check wallet balance")
                        .arg(
                            Arg::new("address")
                                .short('a')
                                .long("address")
                                .help("Address to check balance for")
                                .required(true)
                        )
                        .arg(
                            Arg::new("data-dir")
                                .short('d')
                                .long("data-dir")
                                .help("Data directory")
                                .default_value("./qoranet-data")
                        )
                )
        )
        .subcommand(
            Command::new("transaction")
                .about("Transaction operations")
                .subcommand(
                    Command::new("transfer")
                        .about("Send QOR tokens")
                        .arg(
                            Arg::new("from")
                                .long("from")
                                .help("Sender wallet file")
                                .required(true)
                        )
                        .arg(
                            Arg::new("to")
                                .long("to")
                                .help("Recipient address")
                                .required(true)
                        )
                        .arg(
                            Arg::new("amount")
                                .long("amount")
                                .help("Amount in QOR")
                                .required(true)
                        )
                        .arg(
                            Arg::new("priority")
                                .long("priority")
                                .help("Transaction priority (low, medium, high, urgent)")
                                .default_value("medium")
                        )
                )
                .subcommand(
                    Command::new("fee-estimate")
                        .about("Get fee estimates")
                        .arg(
                            Arg::new("type")
                                .long("type")
                                .help("Transaction type (transfer, liquidity, app, etc.)")
                                .default_value("transfer")
                        )
                )
        )
        .subcommand(
            Command::new("network")
                .about("Network information")
                .subcommand(
                    Command::new("status")
                        .about("Show network status")
                        .arg(
                            Arg::new("data-dir")
                                .short('d')
                                .long("data-dir")
                                .help("Data directory")
                                .default_value("./qoranet-data")
                        )
                )
        )
        .subcommand(
            Command::new("price")
                .about("QOR price information")
        )
        .get_matches();

    match matches.subcommand() {
        Some(("wallet", wallet_matches)) => handle_wallet_commands(wallet_matches).await,
        Some(("transaction", tx_matches)) => handle_transaction_commands(tx_matches).await,
        Some(("network", network_matches)) => handle_network_commands(network_matches).await,
        Some(("price", _)) => handle_price_command().await,
        _ => {
            println!("Use --help for available commands");
            Ok(())
        }
    }
}

async fn handle_wallet_commands(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("generate", gen_matches)) => {
            let output_file = gen_matches.get_one::<String>("output").unwrap();
            generate_wallet(output_file).await
        },
        Some(("balance", balance_matches)) => {
            let address_str = balance_matches.get_one::<String>("address").unwrap();
            let data_dir = balance
