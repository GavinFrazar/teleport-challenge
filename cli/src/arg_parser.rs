use clap::{ArgEnum, Parser, Subcommand};
use uuid::Uuid;

/// Connect to a gRPC job server
#[derive(Debug, Parser)]
pub struct ArgParser {
    /// The address of the server
    #[clap(short = 's', long = "server")]
    pub server: String,
    /// The sub-command to issue
    #[clap(subcommand)]
    pub sub_command: SubCommand,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Subcommand)]
pub enum SubCommand {
    Start {
        #[clap(long)]
        command: String,

        #[clap(long, multiple_values = true)]
        args: Vec<String>,

        #[clap(long, multiple_values = true)]
        dir: Option<String>,

        #[clap(long, multiple_values = true, parse(try_from_str = var_eq_val))]
        envs: Vec<(String, String)>,
    },
    Stop {
        job_id: Uuid,
    },
    Status {
        job_id: Uuid,
    },
    Output {
        #[clap(arg_enum)]
        output_type: OutputType,

        job_id: Uuid,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum OutputType {
    Stdout,
    Stderr,
    All,
}

fn var_eq_val(s: &str) -> Result<(String, String), String> {
    let mut v: Vec<String> = s.split("=").map(str::to_string).collect();
    if v.len() != 2 {
        Err(format!("Required format is VAR=VAL"))
    } else {
        let val = v.pop().unwrap();
        let var = v.pop().unwrap();
        Ok((var, val))
    }
}
