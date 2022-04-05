mod arg_parser;
mod client_cli;

use arg_parser::{ArgParser, SubCommand};
use client_cli::ClientCli;
use protobuf::output_request;

use clap::Parser;
use std::error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let args = ArgParser::parse();
    let mut client = ClientCli::connect(&args.user, &args.server).await;

    // if args.start/stop/status/stream -> do thing
    match args.sub_command {
        SubCommand::Start {
            command,
            args,
            dir,
            envs,
        } => {
            client.start_job(&command, &args, &dir, &envs).await?;
        }
        SubCommand::Stop { job_id } => {
            client.stop_job(job_id).await?;
        }
        SubCommand::Status { job_id } => {
            client.query_status(job_id).await?;
        }
        SubCommand::Output {
            job_id,
            output_type,
        } => {
            let output_type = match output_type {
                arg_parser::OutputType::Stdout => output_request::OutputType::Stdout,
                arg_parser::OutputType::Stderr => output_request::OutputType::Stderr,
                arg_parser::OutputType::All => output_request::OutputType::All,
            };
            client.stream_output(job_id, output_type).await?
        }
    }

    Ok(())
}
