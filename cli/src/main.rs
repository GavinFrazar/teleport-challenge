mod arg_parser;
mod client_cli;

use arg_parser::{ArgParser, SubCommand};
use clap::Parser;
use client_cli::ClientCli;

use tokio;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let user = "charlie"; // TODO: add config in a real implementation

    let args = ArgParser::parse();
    println!("{:?}", args);
    let mut client = ClientCli::connect(user, &args.server).await;

    // if args.start/stop/status/stream -> do thing
    match args.sub_command {
        SubCommand::Start {
            command,
            args,
            dir,
            envs,
        } => {}
        SubCommand::Stop { job_id } => {}
        SubCommand::Status { job_id } => {}
        SubCommand::Output {
            job_id,
            output_type,
        } => {}
    }
}
