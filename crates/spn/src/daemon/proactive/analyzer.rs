//! Suggestion analyzer for generating proactive suggestions.

#![allow(dead_code)]

use super::triggers::{ContextEvent, ContextTrigger, TriggerCondition};
use super::types::{
    ProactiveSuggestion, SuggestionAction, SuggestionCategory, SuggestionId, SuggestionPriority,
    SuggestionSource, SuggestionSummary,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Analyzes context and generates proactive suggestions.
#[derive(Debug)]
pub struct SuggestionAnalyzer {
    /// Registered triggers.
    triggers: Arc<RwLock<Vec<ContextTrigger>>>,
    /// Active suggestions.
    suggestions: Arc<RwLock<FxHashMap<SuggestionId, ProactiveSuggestion>>>,
    /// Maximum active suggestions.
    max_suggestions: usize,
    /// Default suggestion TTL.
    default_ttl: Duration,
}

impl SuggestionAnalyzer {
    /// Create a new analyzer.
    pub fn new() -> Self {
        Self {
            triggers: Arc::new(RwLock::new(Vec::new())),
            suggestions: Arc::new(RwLock::new(FxHashMap::default())),
            max_suggestions: 50,
            default_ttl: Duration::from_secs(3600), // 1 hour
        }
    }

    /// Set max suggestions.
    pub fn with_max_suggestions(mut self, max: usize) -> Self {
        self.max_suggestions = max;
        self
    }

    /// Set default TTL.
    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = ttl;
        self
    }

    /// Initialize with default triggers.
    pub async fn init_defaults(&self) {
        let default_triggers = vec![
            // Suggest tests after code changes
            ContextTrigger::new("run-tests-after-code-change")
                .with_condition(TriggerCondition::file_modified("*.rs"))
                .with_cooldown(Duration::from_secs(300)),
            // Suggest commit after multiple file changes
            ContextTrigger::new("commit-reminder")
                .with_condition(TriggerCondition::GitStatusChanged)
                .with_cooldown(Duration::from_secs(600)),
            // Suggest dependency update after build failure
            ContextTrigger::new("deps-after-build-fail")
                .with_condition(TriggerCondition::BuildCompleted { success: false })
                .with_cooldown(Duration::from_secs(1800)),
            // Suggest format after file save
            ContextTrigger::new("format-reminder")
                .with_condition(TriggerCondition::file_modified("*.rs"))
                .with_cooldown(Duration::from_secs(60)),
        ];

        let mut triggers = self.triggers.write().await;
        triggers.extend(default_triggers);
        debug!(count = triggers.len(), "Initialized default triggers");
    }

    /// Register a trigger.
    pub async fn register_trigger(&self, trigger: ContextTrigger) {
        let mut triggers = self.triggers.write().await;
        triggers.push(trigger);
    }

    /// Process an event and generate suggestions.
    pub async fn process_event(&self, event: ContextEvent) -> Vec<ProactiveSuggestion> {
        let mut generated = Vec::new();
        let mut triggers = self.triggers.write().await;

        for trigger in triggers.iter_mut() {
            if !trigger.can_fire() {
                continue;
            }

            // Check if all conditions match
            let matches = trigger.conditions.iter().all(|c| event.matches(c));
            if matches {
                if let Some(suggestion) = self.generate_suggestion(&trigger.name, &event).await {
                    trigger.record_fire();
                    generated.push(suggestion);
                }
            }
        }

        // Store generated suggestions
        if !generated.is_empty() {
            let mut suggestions = self.suggestions.write().await;
            for suggestion in &generated {
                suggestions.insert(suggestion.id, suggestion.clone());
            }
            self.enforce_limits(&mut suggestions);
        }

        generated
    }

    /// Generate a suggestion based on trigger and event.
    async fn generate_suggestion(
        &self,
        trigger_name: &str,
        event: &ContextEvent,
    ) -> Option<ProactiveSuggestion> {
        match trigger_name {
            "run-tests-after-code-change" => {
                let path = match event {
                    ContextEvent::FileModified { path } => path.clone(),
                    _ => return None,
                };
                Some(
                    ProactiveSuggestion::new(
                        SuggestionCategory::Testing,
                        SuggestionPriority::Medium,
                        "Run tests",
                        format!("You modified {}. Consider running tests.", path),
                        SuggestionSource::FileChange { path },
                    )
                    .with_action(SuggestionAction::run_command("cargo test"))
                    .with_confidence(0.7),
                )
            }

            "commit-reminder" => Some(
                ProactiveSuggestion::new(
                    SuggestionCategory::Workflow,
                    SuggestionPriority::Low,
                    "Commit changes",
                    "You have uncommitted changes. Consider committing.",
                    SuggestionSource::ProjectAnalysis {
                        aspect: "git status".into(),
                    },
                )
                .with_action(SuggestionAction::run_command("git status"))
                .with_confidence(0.6),
            ),

            "deps-after-build-fail" => Some(
                ProactiveSuggestion::new(
                    SuggestionCategory::Quality,
                    SuggestionPriority::High,
                    "Check dependencies",
                    "Build failed. Check if dependencies need updating.",
                    SuggestionSource::ErrorPattern {
                        pattern: "build failure".into(),
                    },
                )
                .with_action(SuggestionAction::run_command("cargo update"))
                .with_confidence(0.5),
            ),

            "format-reminder" => Some(
                ProactiveSuggestion::new(
                    SuggestionCategory::Quality,
                    SuggestionPriority::Info,
                    "Format code",
                    "Run cargo fmt to ensure consistent formatting.",
                    SuggestionSource::Scheduled {
                        trigger: "file save".into(),
                    },
                )
                .with_action(SuggestionAction::run_command("cargo fmt"))
                .with_confidence(0.8),
            ),

            _ => None,
        }
    }

    /// Enforce suggestion limits.
    fn enforce_limits(&self, suggestions: &mut FxHashMap<SuggestionId, ProactiveSuggestion>) {
        // Remove expired suggestions
        suggestions.retain(|_, s| !s.is_expired());

        // If still over limit, remove oldest low-priority
        while suggestions.len() > self.max_suggestions {
            // Find lowest priority, oldest suggestion
            let to_remove = suggestions
                .iter()
                .filter(|(_, s)| s.is_active())
                .min_by(|(_, a), (_, b)| {
                    b.priority
                        .cmp(&a.priority)
                        .then_with(|| a.created_at.cmp(&b.created_at))
                })
                .map(|(id, _)| *id);

            if let Some(id) = to_remove {
                suggestions.remove(&id);
            } else {
                break;
            }
        }
    }

    /// Add a suggestion directly.
    pub async fn add(&self, suggestion: ProactiveSuggestion) -> SuggestionId {
        let id = suggestion.id;
        let mut suggestions = self.suggestions.write().await;
        suggestions.insert(id, suggestion);
        self.enforce_limits(&mut suggestions);
        id
    }

    /// Get a suggestion by ID.
    pub async fn get(&self, id: &SuggestionId) -> Option<ProactiveSuggestion> {
        let suggestions = self.suggestions.read().await;
        suggestions.get(id).cloned()
    }

    /// Mark suggestion as seen.
    pub async fn mark_seen(&self, id: &SuggestionId) -> bool {
        let mut suggestions = self.suggestions.write().await;
        if let Some(suggestion) = suggestions.get_mut(id) {
            suggestion.mark_seen();
            true
        } else {
            false
        }
    }

    /// Mark suggestion as acted upon.
    pub async fn mark_acted(&self, id: &SuggestionId) -> bool {
        let mut suggestions = self.suggestions.write().await;
        if let Some(suggestion) = suggestions.get_mut(id) {
            suggestion.mark_acted();
            info!(?id, "Suggestion acted upon");
            true
        } else {
            false
        }
    }

    /// Dismiss a suggestion.
    pub async fn dismiss(&self, id: &SuggestionId) -> bool {
        let mut suggestions = self.suggestions.write().await;
        if let Some(suggestion) = suggestions.get_mut(id) {
            suggestion.dismiss();
            true
        } else {
            false
        }
    }

    /// List active suggestions.
    pub async fn list_active(&self) -> Vec<SuggestionSummary> {
        let suggestions = self.suggestions.read().await;
        suggestions
            .values()
            .filter(|s| s.is_active())
            .map(SuggestionSummary::from)
            .collect()
    }

    /// List unseen suggestions.
    pub async fn list_unseen(&self) -> Vec<SuggestionSummary> {
        let suggestions = self.suggestions.read().await;
        suggestions
            .values()
            .filter(|s| s.is_active() && !s.seen)
            .map(SuggestionSummary::from)
            .collect()
    }

    /// List by category.
    pub async fn list_by_category(&self, category: SuggestionCategory) -> Vec<SuggestionSummary> {
        let suggestions = self.suggestions.read().await;
        suggestions
            .values()
            .filter(|s| s.is_active() && s.category == category)
            .map(SuggestionSummary::from)
            .collect()
    }

    /// List by priority.
    pub async fn list_by_priority(&self, priority: SuggestionPriority) -> Vec<SuggestionSummary> {
        let suggestions = self.suggestions.read().await;
        suggestions
            .values()
            .filter(|s| s.is_active() && s.priority == priority)
            .map(SuggestionSummary::from)
            .collect()
    }

    /// Get statistics.
    pub async fn stats(&self) -> SuggestionStats {
        let suggestions = self.suggestions.read().await;
        let triggers = self.triggers.read().await;

        let active: Vec<_> = suggestions.values().filter(|s| s.is_active()).collect();

        let mut by_category = FxHashMap::default();
        let mut by_priority = FxHashMap::default();

        for s in &active {
            *by_category.entry(s.category).or_insert(0) += 1;
            *by_priority.entry(s.priority).or_insert(0) += 1;
        }

        SuggestionStats {
            total_count: suggestions.len(),
            active_count: active.len(),
            unseen_count: active.iter().filter(|s| !s.seen).count(),
            dismissed_count: suggestions.values().filter(|s| s.dismissed).count(),
            acted_count: suggestions.values().filter(|s| s.acted).count(),
            trigger_count: triggers.len(),
            by_category,
            by_priority,
        }
    }

    /// Cleanup old suggestions.
    pub async fn cleanup(&self) -> usize {
        let mut suggestions = self.suggestions.write().await;
        let initial = suggestions.len();

        // Remove expired and acted/dismissed
        suggestions.retain(|_, s| s.is_active() || s.seen);

        let removed = initial - suggestions.len();
        if removed > 0 {
            debug!(removed, "Cleaned up suggestions");
        }
        removed
    }

    /// Clear all suggestions.
    pub async fn clear(&self) {
        let mut suggestions = self.suggestions.write().await;
        suggestions.clear();
    }
}

impl Default for SuggestionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about suggestions.
#[derive(Debug, Clone)]
pub struct SuggestionStats {
    /// Total suggestions.
    pub total_count: usize,
    /// Active suggestions.
    pub active_count: usize,
    /// Unseen suggestions.
    pub unseen_count: usize,
    /// Dismissed suggestions.
    pub dismissed_count: usize,
    /// Acted upon suggestions.
    pub acted_count: usize,
    /// Registered trigger count.
    pub trigger_count: usize,
    /// Count by category.
    pub by_category: FxHashMap<SuggestionCategory, usize>,
    /// Count by priority.
    pub by_priority: FxHashMap<SuggestionPriority, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyzer_creation() {
        let analyzer = SuggestionAnalyzer::new();
        let stats = analyzer.stats().await;
        assert_eq!(stats.total_count, 0);
    }

    #[tokio::test]
    async fn test_add_suggestion() {
        let analyzer = SuggestionAnalyzer::new();

        let suggestion = ProactiveSuggestion::new(
            SuggestionCategory::Testing,
            SuggestionPriority::Medium,
            "Test",
            "Description",
            SuggestionSource::Manual,
        );

        let id = analyzer.add(suggestion).await;
        let retrieved = analyzer.get(&id).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_suggestion_lifecycle() {
        let analyzer = SuggestionAnalyzer::new();

        let suggestion = ProactiveSuggestion::new(
            SuggestionCategory::Quality,
            SuggestionPriority::Low,
            "Format",
            "Format code",
            SuggestionSource::Manual,
        );

        let id = analyzer.add(suggestion).await;

        // Initially unseen
        let unseen = analyzer.list_unseen().await;
        assert_eq!(unseen.len(), 1);

        // Mark seen
        analyzer.mark_seen(&id).await;
        let unseen = analyzer.list_unseen().await;
        assert_eq!(unseen.len(), 0);

        // Still active
        let active = analyzer.list_active().await;
        assert_eq!(active.len(), 1);

        // Dismiss
        analyzer.dismiss(&id).await;
        let active = analyzer.list_active().await;
        assert_eq!(active.len(), 0);
    }

    #[tokio::test]
    async fn test_process_event() {
        let analyzer = SuggestionAnalyzer::new();
        analyzer.init_defaults().await;

        let event = ContextEvent::FileModified {
            path: "src/main.rs".into(),
        };

        let suggestions = analyzer.process_event(event).await;
        // Should generate at least one suggestion (run tests, format)
        assert!(!suggestions.is_empty());
    }

    #[tokio::test]
    async fn test_filter_by_category() {
        let analyzer = SuggestionAnalyzer::new();

        analyzer
            .add(ProactiveSuggestion::new(
                SuggestionCategory::Testing,
                SuggestionPriority::Medium,
                "Test 1",
                "Desc",
                SuggestionSource::Manual,
            ))
            .await;

        analyzer
            .add(ProactiveSuggestion::new(
                SuggestionCategory::Security,
                SuggestionPriority::High,
                "Security 1",
                "Desc",
                SuggestionSource::Manual,
            ))
            .await;

        let testing = analyzer.list_by_category(SuggestionCategory::Testing).await;
        assert_eq!(testing.len(), 1);

        let security = analyzer.list_by_category(SuggestionCategory::Security).await;
        assert_eq!(security.len(), 1);
    }

    #[tokio::test]
    async fn test_stats() {
        let analyzer = SuggestionAnalyzer::new();
        analyzer.init_defaults().await;

        analyzer
            .add(ProactiveSuggestion::new(
                SuggestionCategory::Testing,
                SuggestionPriority::High,
                "Test",
                "Desc",
                SuggestionSource::Manual,
            ))
            .await;

        let stats = analyzer.stats().await;
        assert_eq!(stats.total_count, 1);
        assert_eq!(stats.active_count, 1);
        assert!(stats.trigger_count > 0);
    }
}
