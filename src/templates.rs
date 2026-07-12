/// The marker block content for CLAUDE.md.
pub const PREAMBLE: &str = include_str!("../templates/preamble.md");

/// The constitution file seeded into ~/.self/.
pub const CONSTITUTION: &str = include_str!("../templates/constitution.md");

/// Seed content for REGISTRY.md.
pub const REGISTRY: &str = include_str!("../templates/seed/REGISTRY.md");

/// Seed content for observations.md.
pub const OBSERVATIONS: &str = include_str!("../templates/seed/observations.md");

/// Seed content for retired.md.
pub const RETIRED: &str = include_str!("../templates/seed/retired.md");

/// Seed content for log/runs.md.
pub const RUNS: &str = include_str!("../templates/seed/runs.md");

/// Factory content for the SelfLearning agent definition.
pub const SELF_LEARNING: &str = include_str!("../templates/agents/self-learning.md");

/// Factory content for the SelfImproving agent definition.
pub const SELF_IMPROVING: &str = include_str!("../templates/agents/self-improvement.md");
