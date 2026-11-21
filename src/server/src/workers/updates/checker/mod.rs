mod runtime;

pub(crate) use runtime::{check, deadline};

#[cfg(test)]
mod tests;

#[cfg(test)]
mod scheduler_fixture;
