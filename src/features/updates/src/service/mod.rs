mod checks;
mod drain;
mod inspection;
mod installation;
pub use installation::{UpdateInstallCommand, UpdateInstallService};

pub use checks::{
    NoopUpdateChangeNotifier, ReleaseCheckPolicy, ReleaseCheckService, UpdateChangeNotifier,
};
pub use drain::UpdateDrainService;
pub use inspection::ReleaseInspectionService;

#[cfg(test)]
mod tests;
