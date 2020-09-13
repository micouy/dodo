mod deps;
mod error;
mod target;
mod util;

use error::{Error, Result};
use target::Config;
use util::{print_targets, read_config, run_targets};

fn main() -> Result<()> {
    let args = std::env::args().nth(1);
    let dodo = read_config(args)?;
    let dodo = toml::from_str::<Config>(&dodo).map_err(Error::TOML)?;

    print_targets(&dodo.targets)?;
    run_targets(&dodo.targets)?;

    Ok(())
}
