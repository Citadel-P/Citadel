//! Shared job grouping vocabulary, independent of permissions and scheduling.

crate::status_enum! {
    pub enum JobCategory {
        Alert, Build, Backup, Script, GitRepository, Deployment, SwarmService,
        Stack, Image, ResourceCopy, Volume, Container, Updates, Platform,
        Maintenance, Core, Other
    }
}

impl From<JobCategory> for &'static str {
    fn from(category: JobCategory) -> Self {
        category.as_str()
    }
}
