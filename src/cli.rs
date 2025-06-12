use std::path::PathBuf;

#[derive(Debug)]
pub enum Subcommand {
    Inspect(PathBuf),
    Backlinks(PathBuf),
}

/// Parsed ommand-line arguments
#[derive(Debug)]
pub struct Args {
    pub subcommand: Subcommand,
    /// Whether to output the results as json
    pub json: bool,
    pub vault_dir: PathBuf,
}

impl Args {
    /// Parse the arguments from the command line
    pub fn parse() -> Result<Args, lexopt::Error> {
        use lexopt::prelude::*;

        let mut subcommand = None;
        let mut argument = None;
        let mut parser = lexopt::Parser::from_env();
        let mut json = false;
        let mut vault_dir = std::env::current_dir().unwrap();
        while let Some(arg) = parser.next()? {
            match arg {
                Value(val) if subcommand.is_none() => {
                    subcommand = Some(val.clone().string()?);
                }
                Value(val) => {
                    argument = Some(val.string()?);
                }
                Short('j') | Long("json") => {
                    json = true;
                }
                Short('d') | Long("vault-dir") => {
                    let path = parser.value()?.parse::<String>()?.to_string();
                    vault_dir = PathBuf::from(path);
                }
                Short('h') | Long("help") => {
                    println!("Usage: zk [-j|--json] [-d|--vault-dir=DIR] SUBCOMMAND PATH");
                    std::process::exit(0);
                }
                _ => return Err(arg.unexpected()),
            }
        }
        let subcommand = match subcommand.ok_or("missing subcommand")? {
            val if val == "inspect" => {
                Subcommand::Inspect(argument.ok_or("missing argument")?.into())
            }
            val if val == "backlinks" => {
                Subcommand::Backlinks(argument.ok_or("missing argument")?.into())
            }
            _ => todo!(),
        };
        Ok(Args {
            subcommand,
            json,
            vault_dir,
        })
    }
}
