/// The marker block content for CLAUDE.md.
pub const PREAMBLE: &str = include_str!("../templates/preamble.md");

/// The constitution file seeded into ~/.self/.
pub const CONSTITUTION: &str = include_str!("../templates/constitution.md");

/// Seed content for REGISTRY.md (S-0001 date is replaced at runtime).
pub const REGISTRY: &str = include_str!("../templates/seed/REGISTRY.md");

/// Seed content for observations.md.
pub const OBSERVATIONS: &str = include_str!("../templates/seed/observations.md");

/// Seed content for retired.md.
pub const RETIRED: &str = include_str!("../templates/seed/retired.md");

/// Seed content for log/runs.md.
pub const RUNS: &str = include_str!("../templates/seed/runs.md");

/// Factory content for the ci-gate skill.
pub const CI_GATE_SKILL: &str = include_str!("../templates/seed/skills/ci-gate/SKILL.md");

/// Factory content for the self-learner agent definition.
pub const SELF_LEARNER: &str = include_str!("../templates/agents/self-learner.md");

/// Factory content for the self-improver agent definition.
pub const SELF_IMPROVER: &str = include_str!("../templates/agents/self-improver.md");
