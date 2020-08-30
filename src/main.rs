use toml;

mod error;
mod target;
mod util;

use error::{Error, Result};
use target::Config;
use util::{print_targets, read_config};

fn main() -> Result<()> {
    let args = std::env::args().skip(1).next();
    let dodo = read_config(args)?;
    let dodo = toml::from_str::<Config>(&dodo).map_err(Error::TOML)?;

    print_targets(&dodo.targets)?;

    Ok(())
}
