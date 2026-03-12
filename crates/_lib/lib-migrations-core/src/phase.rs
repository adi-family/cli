/// Deployment phase for a migration.
///
/// Migrations are split into phases for safe red-green deployments:
///
/// - **PreDeploy**: Runs BEFORE new code is deployed. Must be backward-compatible
///   with the old code still running. Safe operations: add nullable columns,
///   create new tables, add indexes.
///
/// - **PostDeploy**: Runs AFTER old code is fully terminated. Can include
///   breaking changes. Operations: drop columns, rename columns, add NOT NULL
///   constraints, drop tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Phase {
    /// Runs before new code deployment (must be backward-compatible)
    #[default]
    PreDeploy,
    /// Runs after old code is terminated (can be breaking)
    PostDeploy,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::PreDeploy => "pre-deploy",
            Phase::PostDeploy => "post-deploy",
        }
    }
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Phase {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pre-deploy" | "predeploy" | "pre" => Ok(Phase::PreDeploy),
            "post-deploy" | "postdeploy" | "post" => Ok(Phase::PostDeploy),
            _ => Err(format!("Unknown phase: {}. Use 'pre-deploy' or 'post-deploy'", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_display() {
        assert_eq!(Phase::PreDeploy.to_string(), "pre-deploy");
        assert_eq!(Phase::PostDeploy.to_string(), "post-deploy");
    }

    #[test]
    fn test_phase_parse() {
        assert_eq!("pre-deploy".parse::<Phase>().unwrap(), Phase::PreDeploy);
        assert_eq!("post-deploy".parse::<Phase>().unwrap(), Phase::PostDeploy);
        assert_eq!("pre".parse::<Phase>().unwrap(), Phase::PreDeploy);
        assert_eq!("post".parse::<Phase>().unwrap(), Phase::PostDeploy);
    }

    #[test]
    fn test_phase_default() {
        assert_eq!(Phase::default(), Phase::PreDeploy);
    }
}
