//! Supervised release checking and installation execution.
mod checker;
mod installation;
pub use checker::run;
pub use installation::run_installation;
