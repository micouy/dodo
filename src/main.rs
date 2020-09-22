#![feature(result_flattening)]
#![allow(clippy::iter_nth_zero)]

mod deps;
mod error;
mod target;
mod util;

use deps::DependencyGraph;
use error::{Error, Result};
use target::Config;

fn main() -> Result<()> {
    let target = std::env::args().nth(1).expect("specify target");

    let dodo = util::read_config("dodo.toml")?;
    let dodo = toml::from_str::<Config>(&dodo).map_err(Error::TOML)?;
    let deps = DependencyGraph::construct(dodo.targets.clone())?;
    let target_sequence = deps.get_target_sequence(target.into())?;

    util::print_targets(&dodo.targets)?;
    util::run_targets(target_sequence)?;

    Ok(())
}
