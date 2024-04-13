mod balance;
mod busses;
mod claim;
mod cu_limits;
#[cfg(feature = "admin")]
mod initialize;
mod mine;
mod register;
mod rewards;
mod send_and_confirm;
mod treasury;
#[cfg(feature = "admin")]
mod update_admin;
#[cfg(feature = "admin")]
mod update_difficulty;
mod utils;
mod token_authenticator;


use solana_sdk::signature::{Keypair, read_keypair_file};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use bs58;
use std::sync::Arc;

use clap::{command, Parser, Subcommand};


struct Miner {
    pub auth_filepath: Option<String>,
    pub feepayer_filepath: Option<String>,
    //pub priority_fee: u64,
    pub cluster: String,
    pub jito_fee: u64,
    pub jito_enable: bool,
    pub be_url: String,
}

#[derive(Parser, Debug)]
#[command(about, version)]


struct Args {
    #[arg(
    long,
    value_name = "JitoTips Fee",
    help = "10000=0.00001SOL",
    default_value = "10000"
    )]
    jito_fee: u64,
    #[arg(
    long,
    value_name = "enable JitoTips",
    help = "enable JitoTips?",
    default_value = "false"
    )]
    jito_enable: bool,
    #[arg(
        long,
        value_name = "NETWORK_URL",
        help = "Network address of your RPC provider",
        global = true
    )]
    rpc: Option<String>,
    #[arg(
        long,
        value_name = "JITO_URL",
        help = "Network address of your JITO RPC provider",
        global = true
    )]
    be_url: Option<String>,
    #[clap(
        global = true,
        short = 'C',
        long = "config",
        id = "PATH",
        help = "Filepath to config file."
    )]
    pub config_file: Option<String>,
    #[arg(
        long,
        value_name = "KEYPAIR_FILEPATH",
        help = "Filepath to keypair to use",
        global = true
    )]
    feepayer: Option<String>,
    #[arg(
        long,
        value_name = "MICROLAMPORTS",
        help = "Number of microlamports to pay as priority fee per transaction",
        default_value = "0",
        global = true
    )]
    auth: Option<String>,
    #[arg(
        long,
        value_name = "MICROLAMPORTS",
        help = "Number of microlamports to pay as priority fee per transaction",
        default_value = "0",
        global = true
    )]
    miner1: Option<String>,
        #[arg(
        long,
        value_name = "KEYPAIR2_FILEPATH",
        help = "Filepath to second keypair to use",
        global = true
    )]
    miner2: Option<String>,
    #[arg(
        long,
        value_name = "MICROLAMPORTS",
        help = "Number of microlamports to pay as priority fee per transaction",
        default_value = "0",
        global = true
    )]
    miner3: Option<String>,
    #[arg(
        long,
        value_name = "MICROLAMPORTS",
        help = "Number of microlamports to pay as priority fee per transaction",
        default_value = "0",
        global = true
    )]
    miner4: Option<String>,
    #[arg(
        long,
        value_name = "MICROLAMPORTS",
        help = "Number of microlamports to pay as priority fee per transaction",
        default_value = "0",
        global = true
    )]
    miner5: Option<String>,
    #[arg(
        long,
        value_name = "MICROLAMPORTS",
        help = "Number of microlamports to pay as priority fee per transaction",
        default_value = "0",
        global = true
    )]
    priority_fee: u64,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "Fetch the Ore balance of an account")]
    Balance(BalanceArgs),

    #[command(about = "Fetch the distributable rewards of the busses")]
    Busses(BussesArgs),

    #[command(about = "Mine Ore using local compute")]
    Mine(MineArgs),

    #[command(about = "Claim available mining rewards")]
    Claim(ClaimArgs),

    #[command(about = "Fetch your balance of unclaimed mining rewards")]
    Rewards(RewardsArgs),

    #[command(about = "Fetch the treasury account and balance")]
    Treasury(TreasuryArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Initialize the program")]
    Initialize(InitializeArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Update the program admin authority")]
    UpdateAdmin(UpdateAdminArgs),

    #[cfg(feature = "admin")]
    #[command(about = "Update the mining difficulty")]
    UpdateDifficulty(UpdateDifficultyArgs),
}

#[derive(Parser, Debug)]
struct BalanceArgs {
    #[arg(
        // long,
        value_name = "ADDRESS",
        help = "The address of the account to fetch the balance of"
    )]
    pub address: Option<String>,
}

#[derive(Parser, Debug)]
struct BussesArgs {}

#[derive(Parser, Debug)]
struct RewardsArgs {
    #[arg(
        // long,
        value_name = "ADDRESS",
        help = "The address of the account to fetch the rewards balance of"
    )]
    pub address: Option<String>,
}

#[derive(Parser, Debug)]
struct MineArgs {
    #[arg(
        long,
        short,
        value_name = "THREAD_COUNT",
        help = "The number of threads to dedicate to mining",
        default_value = "1"
    )]
    threads: u64,
}

#[derive(Parser, Debug)]
struct TreasuryArgs {}

#[derive(Parser, Debug)]
struct ClaimArgs {
    #[arg(
        // long,
        value_name = "AMOUNT",
        help = "The amount of rewards to claim. Defaults to max."
    )]
    amount: Option<f64>,

    #[arg(
        // long,
        value_name = "TOKEN_ACCOUNT_ADDRESS",
        help = "Token account to receive mining rewards."
    )]
    beneficiary: Option<String>,
}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
struct InitializeArgs {}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
struct UpdateAdminArgs {
    new_admin: String,
}

#[cfg(feature = "admin")]
#[derive(Parser, Debug)]
struct UpdateDifficultyArgs {}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Load the config file from custom path, the default path, or use default config values
    let cli_config = if let Some(config_file) = &args.config_file {
        solana_cli_config::Config::load(config_file).unwrap_or_else(|_| {
            eprintln!("error: Could not find config file `{}`", config_file);
            std::process::exit(1);
        })
    } else if let Some(config_file) = &*solana_cli_config::CONFIG_FILE {
        solana_cli_config::Config::load(config_file).unwrap_or_default()
    } else {
        solana_cli_config::Config::default()
    };

    // Initialize miner.
    let cluster = args.rpc.unwrap_or(cli_config.json_rpc_url.clone());
    let be_url = args.be_url.unwrap_or(cli_config.json_rpc_url.clone());
    let feepayer_keypair = args.feepayer.unwrap_or(cli_config.keypair_path);
    let auth_keypair = args.auth.unwrap_or(Default::default());

    let miner = Arc::new(Miner::new(
        cluster.clone(),
        be_url.clone(),
        //args.priority_fee,
        Some(feepayer_keypair),
        Some(auth_keypair),
        args.jito_fee,
        args.jito_enable
    ));

    // Execute user command.
    match args.command {
        Commands::Balance(args) => {
            miner.balance(args.address).await;
        }
        Commands::Busses(_) => {
            miner.busses().await;
        }
        Commands::Rewards(args) => {
            miner.rewards(args.address).await;
        }
        Commands::Treasury(_) => {
            miner.treasury().await;
        }
        Commands::Mine(args) => {
            miner.mine(args.threads).await;
        }
        Commands::Claim(args) => {
            miner.claim(cluster, args.beneficiary, args.amount).await;
        }
        #[cfg(feature = "admin")]
        Commands::Initialize(_) => {
            miner.initialize().await;
        }
        #[cfg(feature = "admin")]
        Commands::UpdateAdmin(args) => {
            miner.update_admin(args.new_admin).await;
        }
        #[cfg(feature = "admin")]
        Commands::UpdateDifficulty(_) => {
            miner.update_difficulty().await;
        }
    }
}
impl Miner {
    pub fn new(
        cluster: String, 
        be_url: String, 
        //priority_fee: u64, 
        feepayer_filepath: Option<String>,
        auth_filepath: Option<String>,
        jito_fee: u64, 
        jito_enable: bool) -> Self {

        Self {
            auth_filepath,
            feepayer_filepath,
            //priority_fee,
            cluster,
            be_url,
            jito_fee,
            jito_enable,
        }
    }

    pub fn signers(&self) -> Vec<Keypair> {
        let mut signers = Vec::new();

        if let Ok(lines) = read_lines("keys.txt") {
            for line in lines {
                if let Ok(key) = line {
                    let decoded_key = bs58::decode(key).into_vec().unwrap();
                    let keypair = Keypair::from_bytes(&decoded_key).unwrap();
                    signers.push(keypair);
                }
            }
        }
        signers
    }
    pub fn feepayer(&self) -> Keypair {
        match self.feepayer_filepath.clone() {
            Some(filepath) => read_keypair_file(filepath).unwrap(),
            None => panic!("No keypair provided"),
        }
    }

    pub fn auth(&self) -> Keypair {
        match self.auth_filepath.clone() {
            Some(filepath) => read_keypair_file(filepath).unwrap(),
            None => panic!("No keypair provided"),
        }
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}