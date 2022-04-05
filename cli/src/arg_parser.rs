use clap::{ArgEnum, Parser, Subcommand};
use uuid::Uuid;

/// Connect to a gRPC job server
#[derive(Debug, Parser)]
pub struct ArgParser {
    /// The address of the server
    #[clap(short = 's', long = "server")]
    pub server: String,
    /// The sub-command to use
    #[clap(subcommand)]
    pub sub_command: SubCommand,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Subcommand)]
pub enum SubCommand {
    /// start a new job
    Start {
        #[clap(long)]
        /// name of the command to run
        command: String,

        #[clap(long, multiple_values = true)]
        /// a list of args to the command
        args: Vec<String>,

        #[clap(long)]
        /// working directory for the command
        dir: String,

        #[clap(long, multiple_values = true, parse(try_from_str = var_eq_val))]
        /// list of environment variables
        envs: Vec<(String, String)>,
    },
    /// stop a job
    Stop {
        /// Uuid v4 string
        job_id: Uuid,
    },
    /// get a job's status
    Status {
        /// Uuid v4 string
        job_id: Uuid,
    },
    /// stream a job's output
    Output {
        /// type of output to stream
        #[clap(arg_enum)]
        output_type: OutputType,

        /// Uuid v4 string
        job_id: Uuid,
    },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ArgEnum)]
pub enum OutputType {
    /// stream stdout
    Stdout,
    /// stream stderr
    Stderr,
    /// stream stdout and stderr
    All,
}

/// try_from_str parse function for command env variables
fn var_eq_val(s: &str) -> Result<(String, String), String> {
    let mut v: Vec<String> = s.split('=').map(str::to_string).collect();
    if v.len() != 2 {
        Err("Required format is VAR=VAL".to_string())
    } else {
        let val = v.pop().unwrap();
        let var = v.pop().unwrap();
        Ok((var, val))
    }
}
