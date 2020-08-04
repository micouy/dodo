use serde::*;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct TargetOpts {
    pub working_dir: Option<PathBuf>,
    pub deps: Option<Vec<String>>,
    pub tasks: Vec<Task>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Task {
    command,
}: String,
    working_dir: Option<PathBuf>,
}

impl<'de> Deserialize<'de> for Task {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
                D: Deserializer<'de>,
                    {
                                                 deserializer.deserialize_map(MyMapVisitor::new())
                                                     }
}
