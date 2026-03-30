//! Plugin system — sandboxed WASM-based extensions for Nous nodes.
//!
//! Plugins run in isolated WASM sandboxes with capability-based access control.
//! Each plugin declares what capabilities it needs (network, storage, crypto, etc.)
//! and the host grants or denies them at install time.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// A capability that a plugin can request from the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Capability {
    /// Read from local storage.
    StorageRead,
    /// Write to local storage.
    StorageWrite,
    /// Send network messages.
    NetworkSend,
    /// Receive network messages.
    NetworkReceive,
    /// Access cryptographic operations.
    Crypto,
    /// Access identity information.
    Identity,
    /// Access messaging subsystem.
    Messaging,
    /// Access governance subsystem.
    Governance,
    /// Access payment subsystem.
    Payments,
    /// Access AI inference.
    AiInference,
    /// Access the filesystem (sandboxed to plugin directory).
    Filesystem,
    /// Make HTTP requests to external services.
    HttpClient,
    /// Register custom API endpoints.
    ApiEndpoint,
    /// Register UI components (web, TUI).
    UiExtension,
}

impl Capability {
    /// Whether this capability is considered "dangerous" (requires explicit approval).
    pub fn is_dangerous(&self) -> bool {
        matches!(
            self,
            Capability::StorageWrite
                | Capability::NetworkSend
                | Capability::Payments
                | Capability::HttpClient
                | Capability::Filesystem
        )
    }
}

/// Metadata about a plugin, typically read from a manifest file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier (reverse-domain style).
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Plugin author.
    pub author: String,
    /// Short description.
    pub description: String,
    /// Capabilities this plugin requires.
    pub capabilities: HashSet<Capability>,
    /// Maximum memory the plugin can use (bytes).
    pub max_memory: u64,
    /// Maximum execution time per invocation.
    pub max_execution_time: Duration,
    /// Entry point function name in the WASM module.
    pub entry_point: String,
    /// Plugin hooks — lifecycle events the plugin wants to receive.
    pub hooks: HashSet<Hook>,
}

/// Lifecycle hooks that plugins can subscribe to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Hook {
    /// Called when the node starts.
    OnNodeStart,
    /// Called when the node is shutting down.
    OnNodeShutdown,
    /// Called when a message is received.
    OnMessageReceived,
    /// Called when a message is about to be sent.
    OnMessageSend,
    /// Called when a peer connects.
    OnPeerConnected,
    /// Called when a peer disconnects.
    OnPeerDisconnected,
    /// Called on a periodic timer.
    OnTimer,
    /// Called when a governance proposal is created.
    OnProposalCreated,
    /// Called when a vote is cast.
    OnVoteCast,
    /// Called when a payment is received.
    OnPaymentReceived,
}

/// The runtime state of an installed plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginState {
    /// Installed but not started.
    Installed,
    /// Currently running.
    Running,
    /// Stopped (manually or due to error).
    Stopped,
    /// Disabled by the user.
    Disabled,
    /// Failed to load or crashed.
    Errored,
}

/// Statistics about a plugin's execution.
#[derive(Debug, Clone)]
pub struct PluginStats {
    /// Total invocations.
    pub invocations: u64,
    /// Total execution time across all invocations.
    pub total_execution_time: Duration,
    /// Number of errors.
    pub errors: u64,
    /// Peak memory usage (bytes).
    pub peak_memory: u64,
    /// Last invocation time.
    pub last_invoked: Option<Instant>,
}

impl PluginStats {
    fn new() -> Self {
        Self {
            invocations: 0,
            total_execution_time: Duration::ZERO,
            errors: 0,
            peak_memory: 0,
            last_invoked: None,
        }
    }

    fn record_invocation(&mut self, duration: Duration, memory: u64, errored: bool) {
        self.invocations += 1;
        self.total_execution_time += duration;
        if memory > self.peak_memory {
            self.peak_memory = memory;
        }
        self.last_invoked = Some(Instant::now());
        if errored {
            self.errors += 1;
        }
    }

    /// Average execution time per invocation.
    pub fn avg_execution_time(&self) -> Duration {
        if self.invocations == 0 {
            Duration::ZERO
        } else {
            self.total_execution_time / self.invocations as u32
        }
    }

    /// Error rate as a fraction [0.0, 1.0].
    pub fn error_rate(&self) -> f64 {
        if self.invocations == 0 {
            0.0
        } else {
            self.errors as f64 / self.invocations as f64
        }
    }
}

/// An installed plugin with its manifest, state, and runtime stats.
struct InstalledPlugin {
    manifest: PluginManifest,
    granted_capabilities: HashSet<Capability>,
    state: PluginState,
    stats: PluginStats,
    wasm_bytes: Vec<u8>,
}

/// The plugin manager: installs, manages, and invokes plugins.
pub struct PluginManager {
    plugins: BTreeMap<String, InstalledPlugin>,
    /// Hook → list of plugin IDs subscribed to that hook.
    hook_registry: HashMap<Hook, Vec<String>>,
    /// Auto-approved capabilities (won't prompt the user).
    auto_approved: HashSet<Capability>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: BTreeMap::new(),
            hook_registry: HashMap::new(),
            auto_approved: HashSet::new(),
        }
    }

    /// Set capabilities that are automatically approved for all plugins.
    pub fn auto_approve(&mut self, caps: &[Capability]) {
        self.auto_approved.extend(caps);
    }

    /// Validate a plugin manifest. Returns a list of issues if invalid.
    pub fn validate_manifest(manifest: &PluginManifest) -> Vec<String> {
        let mut issues = Vec::new();

        if manifest.id.is_empty() {
            issues.push("plugin id is empty".to_string());
        }
        if manifest.name.is_empty() {
            issues.push("plugin name is empty".to_string());
        }
        if manifest.version.is_empty() {
            issues.push("plugin version is empty".to_string());
        }
        if manifest.entry_point.is_empty() {
            issues.push("entry point is empty".to_string());
        }
        if manifest.max_memory == 0 {
            issues.push("max_memory must be > 0".to_string());
        }
        if manifest.max_execution_time.is_zero() {
            issues.push("max_execution_time must be > 0".to_string());
        }
        if manifest.capabilities.is_empty() {
            issues.push("plugin requests no capabilities".to_string());
        }

        issues
    }

    /// Install a plugin from its manifest and WASM bytes.
    /// `granted` is the set of capabilities the user has approved.
    /// Returns an error if the plugin is already installed or if validation fails.
    pub fn install(
        &mut self,
        manifest: PluginManifest,
        wasm_bytes: Vec<u8>,
        granted: HashSet<Capability>,
    ) -> Result<(), String> {
        let issues = Self::validate_manifest(&manifest);
        if !issues.is_empty() {
            return Err(format!("invalid manifest: {}", issues.join(", ")));
        }

        if self.plugins.contains_key(&manifest.id) {
            return Err(format!("plugin '{}' is already installed", manifest.id));
        }

        // Merge auto-approved capabilities.
        let mut effective_granted = granted;
        for cap in &manifest.capabilities {
            if self.auto_approved.contains(cap) {
                effective_granted.insert(*cap);
            }
        }

        // Check that all required capabilities are granted.
        let missing: Vec<_> = manifest
            .capabilities
            .iter()
            .filter(|c| !effective_granted.contains(c))
            .collect();
        if !missing.is_empty() {
            return Err(format!("missing capabilities: {:?}", missing));
        }

        // Register hooks.
        let id = manifest.id.clone();
        for hook in &manifest.hooks {
            self.hook_registry
                .entry(*hook)
                .or_default()
                .push(id.clone());
        }

        self.plugins.insert(
            id,
            InstalledPlugin {
                manifest,
                granted_capabilities: effective_granted,
                state: PluginState::Installed,
                stats: PluginStats::new(),
                wasm_bytes,
            },
        );

        Ok(())
    }

    /// Uninstall a plugin.
    pub fn uninstall(&mut self, plugin_id: &str) -> Result<(), String> {
        if self.plugins.remove(plugin_id).is_none() {
            return Err(format!("plugin '{}' not found", plugin_id));
        }

        // Remove from hook registry.
        for subscribers in self.hook_registry.values_mut() {
            subscribers.retain(|id| id != plugin_id);
        }

        Ok(())
    }

    /// Start a plugin (transition to Running state).
    pub fn start(&mut self, plugin_id: &str) -> Result<(), String> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| format!("plugin '{}' not found", plugin_id))?;

        match plugin.state {
            PluginState::Running => return Err("plugin is already running".to_string()),
            PluginState::Disabled => return Err("plugin is disabled".to_string()),
            _ => {}
        }

        plugin.state = PluginState::Running;
        Ok(())
    }

    /// Stop a running plugin.
    pub fn stop(&mut self, plugin_id: &str) -> Result<(), String> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| format!("plugin '{}' not found", plugin_id))?;

        if plugin.state != PluginState::Running {
            return Err("plugin is not running".to_string());
        }

        plugin.state = PluginState::Stopped;
        Ok(())
    }

    /// Disable a plugin (prevents starting).
    pub fn disable(&mut self, plugin_id: &str) -> Result<(), String> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| format!("plugin '{}' not found", plugin_id))?;

        plugin.state = PluginState::Disabled;
        Ok(())
    }

    /// Enable a disabled plugin (transitions to Stopped).
    pub fn enable(&mut self, plugin_id: &str) -> Result<(), String> {
        let plugin = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| format!("plugin '{}' not found", plugin_id))?;

        if plugin.state != PluginState::Disabled {
            return Err("plugin is not disabled".to_string());
        }

        plugin.state = PluginState::Stopped;
        Ok(())
    }

    /// Record a plugin invocation result.
    pub fn record_invocation(
        &mut self,
        plugin_id: &str,
        duration: Duration,
        memory: u64,
        errored: bool,
    ) {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.stats.record_invocation(duration, memory, errored);
            if errored && plugin.stats.error_rate() > 0.5 && plugin.stats.invocations >= 10 {
                // Auto-stop plugins with high error rates.
                plugin.state = PluginState::Errored;
            }
        }
    }

    /// Check if a plugin has a specific capability.
    pub fn has_capability(&self, plugin_id: &str, cap: Capability) -> bool {
        self.plugins
            .get(plugin_id)
            .is_some_and(|p| p.granted_capabilities.contains(&cap))
    }

    /// Get the state of a plugin.
    pub fn state(&self, plugin_id: &str) -> Option<PluginState> {
        self.plugins.get(plugin_id).map(|p| p.state)
    }

    /// Get the manifest of a plugin.
    pub fn manifest(&self, plugin_id: &str) -> Option<&PluginManifest> {
        self.plugins.get(plugin_id).map(|p| &p.manifest)
    }

    /// Get the stats of a plugin.
    pub fn stats(&self, plugin_id: &str) -> Option<&PluginStats> {
        self.plugins.get(plugin_id).map(|p| &p.stats)
    }

    /// Get the WASM bytes of a plugin.
    pub fn wasm_bytes(&self, plugin_id: &str) -> Option<&[u8]> {
        self.plugins.get(plugin_id).map(|p| p.wasm_bytes.as_slice())
    }

    /// List all installed plugin IDs.
    pub fn list(&self) -> Vec<&str> {
        self.plugins.keys().map(|k| k.as_str()).collect()
    }

    /// List plugins subscribed to a specific hook.
    pub fn plugins_for_hook(&self, hook: Hook) -> Vec<&str> {
        self.hook_registry
            .get(&hook)
            .map(|ids| {
                ids.iter()
                    .filter(|id| {
                        self.plugins
                            .get(id.as_str())
                            .is_some_and(|p| p.state == PluginState::Running)
                    })
                    .map(|id| id.as_str())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Number of installed plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Dangerous capabilities requested by a plugin (for UI display).
    pub fn dangerous_capabilities(manifest: &PluginManifest) -> Vec<Capability> {
        manifest
            .capabilities
            .iter()
            .filter(|c| c.is_dangerous())
            .copied()
            .collect()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create a test manifest.
#[cfg(test)]
fn test_manifest(id: &str) -> PluginManifest {
    PluginManifest {
        id: id.to_string(),
        name: format!("Test Plugin {id}"),
        version: "1.0.0".to_string(),
        author: "test".to_string(),
        description: "A test plugin".to_string(),
        capabilities: HashSet::from([Capability::StorageRead, Capability::Crypto]),
        max_memory: 1024 * 1024, // 1 MiB
        max_execution_time: Duration::from_secs(5),
        entry_point: "main".to_string(),
        hooks: HashSet::from([Hook::OnNodeStart]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_caps(manifest: &PluginManifest) -> HashSet<Capability> {
        manifest.capabilities.clone()
    }

    #[test]
    fn validate_good_manifest() {
        let manifest = test_manifest("com.example.test");
        assert!(PluginManager::validate_manifest(&manifest).is_empty());
    }

    #[test]
    fn validate_rejects_empty_id() {
        let mut manifest = test_manifest("");
        manifest.id = String::new();
        let issues = PluginManager::validate_manifest(&manifest);
        assert!(issues.iter().any(|i| i.contains("id")));
    }

    #[test]
    fn validate_rejects_zero_memory() {
        let mut manifest = test_manifest("test");
        manifest.max_memory = 0;
        let issues = PluginManager::validate_manifest(&manifest);
        assert!(issues.iter().any(|i| i.contains("max_memory")));
    }

    #[test]
    fn validate_rejects_no_capabilities() {
        let mut manifest = test_manifest("test");
        manifest.capabilities.clear();
        let issues = PluginManager::validate_manifest(&manifest);
        assert!(issues.iter().any(|i| i.contains("capabilities")));
    }

    #[test]
    fn install_and_list() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.alpha");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![0x00, 0x61], caps).unwrap();

        assert_eq!(mgr.count(), 1);
        assert_eq!(mgr.list(), vec!["com.example.alpha"]);
    }

    #[test]
    fn install_duplicate_fails() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.dup");
        let caps = all_caps(&manifest);
        mgr.install(manifest.clone(), vec![], caps.clone()).unwrap();
        assert!(mgr.install(manifest, vec![], caps).is_err());
    }

    #[test]
    fn install_with_missing_capabilities_fails() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.missing");
        // Grant only one of the two required capabilities.
        let partial = HashSet::from([Capability::StorageRead]);
        assert!(mgr.install(manifest, vec![], partial).is_err());
    }

    #[test]
    fn auto_approve_fills_gaps() {
        let mut mgr = PluginManager::new();
        mgr.auto_approve(&[Capability::Crypto]);

        let manifest = test_manifest("com.example.auto");
        // Only grant StorageRead; Crypto is auto-approved.
        let partial = HashSet::from([Capability::StorageRead]);
        mgr.install(manifest, vec![], partial).unwrap();

        assert!(mgr.has_capability("com.example.auto", Capability::Crypto));
    }

    #[test]
    fn uninstall() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.remove");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![], caps).unwrap();

        mgr.uninstall("com.example.remove").unwrap();
        assert_eq!(mgr.count(), 0);
    }

    #[test]
    fn uninstall_nonexistent_fails() {
        let mut mgr = PluginManager::new();
        assert!(mgr.uninstall("com.example.nope").is_err());
    }

    #[test]
    fn lifecycle_states() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.lifecycle");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![], caps).unwrap();

        assert_eq!(
            mgr.state("com.example.lifecycle"),
            Some(PluginState::Installed)
        );

        mgr.start("com.example.lifecycle").unwrap();
        assert_eq!(
            mgr.state("com.example.lifecycle"),
            Some(PluginState::Running)
        );

        mgr.stop("com.example.lifecycle").unwrap();
        assert_eq!(
            mgr.state("com.example.lifecycle"),
            Some(PluginState::Stopped)
        );

        mgr.disable("com.example.lifecycle").unwrap();
        assert_eq!(
            mgr.state("com.example.lifecycle"),
            Some(PluginState::Disabled)
        );

        // Can't start when disabled.
        assert!(mgr.start("com.example.lifecycle").is_err());

        mgr.enable("com.example.lifecycle").unwrap();
        assert_eq!(
            mgr.state("com.example.lifecycle"),
            Some(PluginState::Stopped)
        );

        mgr.start("com.example.lifecycle").unwrap();
        assert_eq!(
            mgr.state("com.example.lifecycle"),
            Some(PluginState::Running)
        );
    }

    #[test]
    fn start_already_running_fails() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.running");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![], caps).unwrap();
        mgr.start("com.example.running").unwrap();
        assert!(mgr.start("com.example.running").is_err());
    }

    #[test]
    fn stop_not_running_fails() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.notrunning");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![], caps).unwrap();
        assert!(mgr.stop("com.example.notrunning").is_err());
    }

    #[test]
    fn hook_registry() {
        let mut mgr = PluginManager::new();

        let mut m1 = test_manifest("com.example.hook1");
        m1.hooks = HashSet::from([Hook::OnMessageReceived, Hook::OnPeerConnected]);
        let caps1 = all_caps(&m1);
        mgr.install(m1, vec![], caps1).unwrap();
        mgr.start("com.example.hook1").unwrap();

        let mut m2 = test_manifest("com.example.hook2");
        m2.hooks = HashSet::from([Hook::OnMessageReceived]);
        let caps2 = all_caps(&m2);
        mgr.install(m2, vec![], caps2).unwrap();
        mgr.start("com.example.hook2").unwrap();

        let msg_hooks = mgr.plugins_for_hook(Hook::OnMessageReceived);
        assert_eq!(msg_hooks.len(), 2);

        let peer_hooks = mgr.plugins_for_hook(Hook::OnPeerConnected);
        assert_eq!(peer_hooks.len(), 1);

        let timer_hooks = mgr.plugins_for_hook(Hook::OnTimer);
        assert!(timer_hooks.is_empty());
    }

    #[test]
    fn hooks_only_include_running_plugins() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.stopped");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![], caps).unwrap();
        // Not started — should not appear in hook list.
        assert!(mgr.plugins_for_hook(Hook::OnNodeStart).is_empty());

        mgr.start("com.example.stopped").unwrap();
        assert_eq!(mgr.plugins_for_hook(Hook::OnNodeStart).len(), 1);

        mgr.stop("com.example.stopped").unwrap();
        assert!(mgr.plugins_for_hook(Hook::OnNodeStart).is_empty());
    }

    #[test]
    fn uninstall_removes_from_hooks() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.unhook");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![], caps).unwrap();
        mgr.start("com.example.unhook").unwrap();

        assert_eq!(mgr.plugins_for_hook(Hook::OnNodeStart).len(), 1);

        mgr.uninstall("com.example.unhook").unwrap();
        assert!(mgr.plugins_for_hook(Hook::OnNodeStart).is_empty());
    }

    #[test]
    fn stats_tracking() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.stats");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![], caps).unwrap();
        mgr.start("com.example.stats").unwrap();

        mgr.record_invocation("com.example.stats", Duration::from_millis(10), 512, false);
        mgr.record_invocation("com.example.stats", Duration::from_millis(20), 1024, false);
        mgr.record_invocation("com.example.stats", Duration::from_millis(5), 256, true);

        let stats = mgr.stats("com.example.stats").unwrap();
        assert_eq!(stats.invocations, 3);
        assert_eq!(stats.errors, 1);
        assert_eq!(stats.peak_memory, 1024);
        assert_eq!(stats.total_execution_time, Duration::from_millis(35));
        assert!(stats.avg_execution_time() > Duration::from_millis(10));
        assert!((stats.error_rate() - 1.0 / 3.0).abs() < 0.01);
    }

    #[test]
    fn high_error_rate_auto_stops() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.crashy");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![], caps).unwrap();
        mgr.start("com.example.crashy").unwrap();

        // Record 10 invocations with >50% error rate.
        for i in 0..10 {
            mgr.record_invocation(
                "com.example.crashy",
                Duration::from_millis(1),
                100,
                i % 2 == 0, // 50% errors
            );
        }

        // Should not auto-stop at exactly 50%.
        assert_eq!(mgr.state("com.example.crashy"), Some(PluginState::Running));

        // Push over 50%.
        mgr.record_invocation("com.example.crashy", Duration::from_millis(1), 100, true);
        assert_eq!(mgr.state("com.example.crashy"), Some(PluginState::Errored));
    }

    #[test]
    fn manifest_serializes() {
        let manifest = test_manifest("com.example.serde");
        let json = serde_json::to_string(&manifest).unwrap();
        let restored: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.id, manifest.id);
        assert_eq!(restored.capabilities, manifest.capabilities);
    }

    #[test]
    fn dangerous_capabilities_detection() {
        let mut manifest = test_manifest("test");
        manifest.capabilities = HashSet::from([
            Capability::StorageRead,
            Capability::StorageWrite,
            Capability::Crypto,
            Capability::HttpClient,
        ]);

        let dangerous = PluginManager::dangerous_capabilities(&manifest);
        assert!(dangerous.contains(&Capability::StorageWrite));
        assert!(dangerous.contains(&Capability::HttpClient));
        assert!(!dangerous.contains(&Capability::StorageRead));
        assert!(!dangerous.contains(&Capability::Crypto));
    }

    #[test]
    fn wasm_bytes_stored() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.wasm");
        let caps = all_caps(&manifest);
        let bytes = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic bytes
        mgr.install(manifest, bytes.clone(), caps).unwrap();

        assert_eq!(mgr.wasm_bytes("com.example.wasm"), Some(bytes.as_slice()));
    }

    #[test]
    fn get_manifest() {
        let mut mgr = PluginManager::new();
        let manifest = test_manifest("com.example.manifest");
        let caps = all_caps(&manifest);
        mgr.install(manifest, vec![], caps).unwrap();

        let m = mgr.manifest("com.example.manifest").unwrap();
        assert_eq!(m.name, "Test Plugin com.example.manifest");
    }

    #[test]
    fn nonexistent_plugin_returns_none() {
        let mgr = PluginManager::new();
        assert!(mgr.state("nope").is_none());
        assert!(mgr.manifest("nope").is_none());
        assert!(mgr.stats("nope").is_none());
        assert!(mgr.wasm_bytes("nope").is_none());
    }

    #[test]
    fn capability_is_dangerous() {
        assert!(Capability::StorageWrite.is_dangerous());
        assert!(Capability::NetworkSend.is_dangerous());
        assert!(Capability::Payments.is_dangerous());
        assert!(Capability::HttpClient.is_dangerous());
        assert!(Capability::Filesystem.is_dangerous());
        assert!(!Capability::StorageRead.is_dangerous());
        assert!(!Capability::Crypto.is_dangerous());
        assert!(!Capability::Identity.is_dangerous());
    }
}
