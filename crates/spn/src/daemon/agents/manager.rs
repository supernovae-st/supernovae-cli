//! Agent manager for coordinating delegated agents.

#![allow(dead_code)]

use super::types::{Agent, AgentConfig, AgentId, AgentResult, AgentState, AgentStatus, DelegatedTask};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Manages agent lifecycle and coordination.
#[derive(Debug)]
pub struct AgentManager {
    /// Active agents.
    agents: Arc<RwLock<FxHashMap<AgentId, Agent>>>,
    /// Maximum concurrent agents.
    max_concurrent: usize,
    /// Maximum agent depth (nested delegation).
    max_depth: usize,
}

impl AgentManager {
    /// Create a new agent manager.
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(FxHashMap::default())),
            max_concurrent: 10,
            max_depth: 3,
        }
    }

    /// Set maximum concurrent agents.
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Set maximum nesting depth.
    pub fn with_max_depth(mut self, max: usize) -> Self {
        self.max_depth = max;
        self
    }

    /// Spawn a new agent.
    pub async fn spawn(&self, config: AgentConfig) -> Result<AgentId, AgentError> {
        // Check concurrent limit
        let agents = self.agents.read().await;
        let active_count = agents.values().filter(|a| a.state.is_active()).count();
        if active_count >= self.max_concurrent {
            return Err(AgentError::ConcurrencyLimit(self.max_concurrent));
        }

        // Check depth limit
        if let Some(parent_id) = config.parent_id {
            let depth = self.get_depth(&agents, &parent_id);
            if depth >= self.max_depth {
                return Err(AgentError::DepthLimit(self.max_depth));
            }
        }
        drop(agents);

        // Create and register agent
        let agent = Agent::new(config);
        let id = agent.id;

        let mut agents = self.agents.write().await;
        agents.insert(id, agent);

        info!(?id, "Spawned new agent");
        Ok(id)
    }

    /// Get nesting depth of an agent.
    fn get_depth(&self, agents: &FxHashMap<AgentId, Agent>, id: &AgentId) -> usize {
        let mut depth = 0;
        let mut current_id = *id;

        while let Some(agent) = agents.get(&current_id) {
            if let Some(parent_id) = agent.config.parent_id {
                depth += 1;
                current_id = parent_id;
            } else {
                break;
            }
        }

        depth
    }

    /// Delegate a task to an agent.
    pub async fn delegate(&self, agent_id: AgentId, task: DelegatedTask) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        let agent = agents.get_mut(&agent_id).ok_or(AgentError::NotFound(agent_id))?;

        if agent.state != AgentState::Idle {
            return Err(AgentError::NotIdle(agent_id));
        }

        agent.assign(task);
        debug!(?agent_id, "Delegated task to agent");
        Ok(())
    }

    /// Update agent state.
    pub async fn update_state(&self, agent_id: AgentId, state: AgentState) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        let agent = agents.get_mut(&agent_id).ok_or(AgentError::NotFound(agent_id))?;
        agent.set_state(state);
        Ok(())
    }

    /// Record a turn for an agent.
    pub async fn record_turn(&self, agent_id: AgentId, tokens: u32) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        let agent = agents.get_mut(&agent_id).ok_or(AgentError::NotFound(agent_id))?;
        agent.record_turn(tokens);
        Ok(())
    }

    /// Complete an agent with result.
    pub async fn complete(&self, agent_id: AgentId, result: AgentResult) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        let agent = agents.get_mut(&agent_id).ok_or(AgentError::NotFound(agent_id))?;
        agent.complete(result);
        info!(?agent_id, "Agent completed");
        Ok(())
    }

    /// Fail an agent.
    pub async fn fail(&self, agent_id: AgentId, error: impl Into<String>) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        let agent = agents.get_mut(&agent_id).ok_or(AgentError::NotFound(agent_id))?;
        agent.fail(error);
        warn!(?agent_id, "Agent failed");
        Ok(())
    }

    /// Cancel an agent.
    pub async fn cancel(&self, agent_id: AgentId) -> Result<(), AgentError> {
        let mut agents = self.agents.write().await;
        let agent = agents.get_mut(&agent_id).ok_or(AgentError::NotFound(agent_id))?;
        agent.cancel();
        info!(?agent_id, "Agent cancelled");
        Ok(())
    }

    /// Get agent by ID.
    pub async fn get(&self, agent_id: &AgentId) -> Option<Agent> {
        let agents = self.agents.read().await;
        agents.get(agent_id).cloned()
    }

    /// Get agent status.
    pub async fn status(&self, agent_id: &AgentId) -> Option<AgentStatus> {
        let agents = self.agents.read().await;
        agents.get(agent_id).map(|a| a.status())
    }

    /// List all agents.
    pub async fn list(&self) -> Vec<AgentStatus> {
        let agents = self.agents.read().await;
        agents.values().map(|a| a.status()).collect()
    }

    /// List active agents.
    pub async fn list_active(&self) -> Vec<AgentStatus> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.state.is_active())
            .map(|a| a.status())
            .collect()
    }

    /// Get child agents of a parent.
    pub async fn children(&self, parent_id: &AgentId) -> Vec<AgentStatus> {
        let agents = self.agents.read().await;

        agents
            .values()
            .filter(|a| a.config.parent_id.as_ref() == Some(parent_id))
            .map(|a| a.status())
            .collect()
    }

    /// Spawn a child agent.
    pub async fn spawn_child(
        &self,
        parent_id: AgentId,
        config: AgentConfig,
    ) -> Result<AgentId, AgentError> {
        let config = config.with_parent(parent_id);
        let child_id = self.spawn(config).await?;

        // Register child with parent
        let mut agents = self.agents.write().await;
        if let Some(parent) = agents.get_mut(&parent_id) {
            parent.add_child(child_id);
        }

        Ok(child_id)
    }

    /// Check for timed out agents.
    pub async fn check_timeouts(&self) -> Vec<AgentId> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.state.is_active() && a.is_timed_out())
            .map(|a| a.id)
            .collect()
    }

    /// Check for turn-limited agents.
    pub async fn check_turn_limits(&self) -> Vec<AgentId> {
        let agents = self.agents.read().await;
        agents
            .values()
            .filter(|a| a.state.is_active() && a.is_turn_limited())
            .map(|a| a.id)
            .collect()
    }

    /// Cleanup completed agents older than duration.
    pub async fn cleanup(&self, older_than: std::time::Duration) -> usize {
        let cutoff = std::time::SystemTime::now() - older_than;
        let mut agents = self.agents.write().await;

        let to_remove: Vec<_> = agents
            .iter()
            .filter(|(_, a)| a.state.is_terminal() && a.updated_at < cutoff)
            .map(|(id, _)| *id)
            .collect();

        let count = to_remove.len();
        for id in to_remove {
            agents.remove(&id);
        }

        if count > 0 {
            debug!(count, "Cleaned up completed agents");
        }

        count
    }

    /// Get statistics.
    pub async fn stats(&self) -> AgentStats {
        let agents = self.agents.read().await;

        let mut total_tokens = 0u32;
        let mut total_turns = 0u32;
        let mut by_role = FxHashMap::default();
        let mut by_state = FxHashMap::default();

        for agent in agents.values() {
            total_tokens += agent.tokens_used;
            total_turns += agent.turns_used;
            *by_role.entry(agent.config.role).or_insert(0) += 1;
            *by_state.entry(agent.state).or_insert(0) += 1;
        }

        AgentStats {
            total_count: agents.len(),
            active_count: agents.values().filter(|a| a.state.is_active()).count(),
            total_tokens,
            total_turns,
            by_role,
            by_state,
        }
    }
}

impl Default for AgentManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent management errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AgentError {
    /// Agent not found.
    #[error("Agent not found: {0}")]
    NotFound(AgentId),

    /// Agent not idle.
    #[error("Agent {0} is not idle")]
    NotIdle(AgentId),

    /// Concurrency limit reached.
    #[error("Maximum concurrent agents reached: {0}")]
    ConcurrencyLimit(usize),

    /// Depth limit reached.
    #[error("Maximum agent depth reached: {0}")]
    DepthLimit(usize),
}

/// Agent statistics.
#[derive(Debug, Clone)]
pub struct AgentStats {
    /// Total agent count.
    pub total_count: usize,
    /// Active agent count.
    pub active_count: usize,
    /// Total tokens used.
    pub total_tokens: u32,
    /// Total turns used.
    pub total_turns: u32,
    /// Count by role.
    pub by_role: FxHashMap<super::types::AgentRole, usize>,
    /// Count by state.
    pub by_state: FxHashMap<AgentState, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::agents::types::AgentRole;

    #[tokio::test]
    async fn test_agent_manager_spawn() {
        let manager = AgentManager::new();

        let config = AgentConfig::for_role(AgentRole::Explorer);
        let id = manager.spawn(config).await.unwrap();

        let agent = manager.get(&id).await;
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().state, AgentState::Idle);
    }

    #[tokio::test]
    async fn test_agent_manager_delegate() {
        let manager = AgentManager::new();

        let config = AgentConfig::for_role(AgentRole::Generator);
        let id = manager.spawn(config).await.unwrap();

        let task = DelegatedTask::new("Generate code");
        manager.delegate(id, task).await.unwrap();

        let status = manager.status(&id).await.unwrap();
        assert_eq!(status.state, AgentState::Thinking);
    }

    #[tokio::test]
    async fn test_agent_manager_complete() {
        let manager = AgentManager::new();

        let id = manager.spawn(AgentConfig::default()).await.unwrap();
        manager.delegate(id, DelegatedTask::new("Task")).await.unwrap();
        manager.complete(id, AgentResult::Text("Done".into())).await.unwrap();

        let agent = manager.get(&id).await.unwrap();
        assert_eq!(agent.state, AgentState::Completed);
    }

    #[tokio::test]
    async fn test_agent_manager_concurrency_limit() {
        let manager = AgentManager::new().with_max_concurrent(2);

        // Spawn 2 agents and make them active
        let id1 = manager.spawn(AgentConfig::default()).await.unwrap();
        let id2 = manager.spawn(AgentConfig::default()).await.unwrap();
        manager.delegate(id1, DelegatedTask::new("Task 1")).await.unwrap();
        manager.delegate(id2, DelegatedTask::new("Task 2")).await.unwrap();

        // Third should fail
        let result = manager.spawn(AgentConfig::default()).await;
        assert!(matches!(result, Err(AgentError::ConcurrencyLimit(2))));
    }

    #[tokio::test]
    async fn test_agent_manager_child_spawn() {
        let manager = AgentManager::new();

        let parent_id = manager.spawn(AgentConfig::default()).await.unwrap();
        let child_id = manager
            .spawn_child(parent_id, AgentConfig::for_role(AgentRole::Tester))
            .await
            .unwrap();

        let children = manager.children(&parent_id).await;
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, child_id);
    }

    #[tokio::test]
    async fn test_agent_manager_depth_limit() {
        let manager = AgentManager::new().with_max_depth(2);

        let id1 = manager.spawn(AgentConfig::default()).await.unwrap();
        let id2 = manager.spawn_child(id1, AgentConfig::default()).await.unwrap();
        let id3 = manager.spawn_child(id2, AgentConfig::default()).await.unwrap();

        // Fourth level should fail
        let result = manager.spawn_child(id3, AgentConfig::default()).await;
        assert!(matches!(result, Err(AgentError::DepthLimit(2))));
    }

    #[tokio::test]
    async fn test_agent_manager_stats() {
        let manager = AgentManager::new();

        manager.spawn(AgentConfig::for_role(AgentRole::Explorer)).await.unwrap();
        manager.spawn(AgentConfig::for_role(AgentRole::Explorer)).await.unwrap();
        manager.spawn(AgentConfig::for_role(AgentRole::Generator)).await.unwrap();

        let stats = manager.stats().await;
        assert_eq!(stats.total_count, 3);
        assert_eq!(stats.by_role.get(&AgentRole::Explorer), Some(&2));
        assert_eq!(stats.by_role.get(&AgentRole::Generator), Some(&1));
    }
}
