use std::collections::{BTreeMap, BTreeSet};

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};
use wasmtime_wasi::WasiCtxBuilder;
use wasmtime_wasi::p1::{self, WasiP1Ctx};

const MAX_PLUGIN_BYTES: usize = 1024 * 1024;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExtensionError {
    #[error("extension record or transition is invalid")]
    Invalid,
    #[error("extension identifier already exists")]
    Duplicate,
    #[error("extension capability is not granted")]
    CapabilityDenied,
    #[error("extension signature verification failed")]
    Signature,
    #[error("registry or MCP schema is frozen")]
    Frozen,
    #[error("MCP schema does not match the frozen definition")]
    SchemaMutation,
    #[error("MCP rate limit exhausted")]
    RateLimited,
    #[error("WASI execution failed: {0}")]
    Wasi(String),
    #[error("extension record was not found")]
    NotFound,
}

fn valid_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-' || byte == b'.'
        })
}

fn valid_capability(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':' | b'/')
        })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillStatus {
    Proposed,
    Staged,
    StaticallyChecked,
    TestedInSandbox,
    Evaluated,
    Approved,
    Trusted,
    Rejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SkillRecord {
    pub id: String,
    pub source_digest: [u8; 32],
    pub tests_digest: [u8; 32],
    pub status: SkillStatus,
    pub approver: Option<String>,
    pub approval_signature: Option<[u8; 64]>,
}

impl SkillRecord {
    pub fn new(id: &str, source_digest: [u8; 32], tests_digest: [u8; 32]) -> Self {
        Self {
            id: id.to_owned(),
            source_digest,
            tests_digest,
            status: SkillStatus::Proposed,
            approver: None,
            approval_signature: None,
        }
    }

    pub fn bind_approval(
        &mut self,
        approver: &str,
        signature: [u8; 64],
    ) -> Result<(), ExtensionError> {
        if self.status != SkillStatus::Approved || !valid_label(approver) || signature == [0; 64] {
            return Err(ExtensionError::Invalid);
        }
        self.approver = Some(approver.to_owned());
        self.approval_signature = Some(signature);
        Ok(())
    }

    pub fn transition(&mut self, target: SkillStatus) -> Result<(), ExtensionError> {
        let allowed = matches!(
            (self.status, target),
            (SkillStatus::Proposed, SkillStatus::Staged)
                | (SkillStatus::Staged, SkillStatus::StaticallyChecked)
                | (SkillStatus::StaticallyChecked, SkillStatus::TestedInSandbox)
                | (SkillStatus::TestedInSandbox, SkillStatus::Evaluated)
                | (SkillStatus::Evaluated, SkillStatus::Approved)
        ) || (self.status == SkillStatus::Approved
            && target == SkillStatus::Trusted
            && self.approver.is_some()
            && self.approval_signature.is_some())
            || (target == SkillStatus::Rejected && self.status != SkillStatus::Trusted);
        if !allowed {
            return Err(ExtensionError::Invalid);
        }
        self.status = target;
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginCapabilities {
    pub network_hosts: BTreeSet<String>,
    pub filesystem_roots: BTreeSet<String>,
    pub secrets: BTreeSet<String>,
    pub events: BTreeSet<String>,
}

impl PluginCapabilities {
    fn is_subset_of(&self, granted: &Self) -> bool {
        self.network_hosts.is_subset(&granted.network_hosts)
            && self.filesystem_roots.is_subset(&granted.filesystem_roots)
            && self.secrets.is_subset(&granted.secrets)
            && self.events.is_subset(&granted.events)
    }

    fn is_empty(&self) -> bool {
        self.network_hosts.is_empty()
            && self.filesystem_roots.is_empty()
            && self.secrets.is_empty()
            && self.events.is_empty()
    }

    fn is_valid(&self) -> bool {
        self.network_hosts
            .iter()
            .all(|value| valid_capability(value))
            && self
                .filesystem_roots
                .iter()
                .all(|value| valid_capability(value))
            && self.secrets.iter().all(|value| valid_capability(value))
            && self.events.iter().all(|value| valid_capability(value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginKind {
    WasiBackend,
    UiWebview,
    NativeSubprocess,
    Mcp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsignedPluginManifest {
    pub id: String,
    pub version: String,
    pub protocol: String,
    pub kind: PluginKind,
    pub module_digest: [u8; 32],
    pub capabilities: PluginCapabilities,
    pub evidence_types: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignedPluginManifest {
    pub manifest: UnsignedPluginManifest,
    pub public_key: [u8; 32],
    pub signature: [u8; 64],
}

impl UnsignedPluginManifest {
    fn canonical(&self) -> Result<Vec<u8>, ExtensionError> {
        serde_json::to_vec(self).map_err(|_| ExtensionError::Invalid)
    }

    pub fn sign(self, signing_key: &SigningKey) -> Result<SignedPluginManifest, ExtensionError> {
        if !valid_label(&self.id)
            || !valid_label(&self.version)
            || self.protocol != "nemesis.plugin/v1"
            || !self.capabilities.is_valid()
            || self.evidence_types.iter().any(|value| !valid_label(value))
        {
            return Err(ExtensionError::Invalid);
        }
        let signature = signing_key.sign(&self.canonical()?).to_bytes();
        Ok(SignedPluginManifest {
            manifest: self,
            public_key: signing_key.verifying_key().to_bytes(),
            signature,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct PluginRegistry {
    plugins: BTreeMap<String, SignedPluginManifest>,
    data: BTreeMap<String, Vec<[u8; 32]>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn install(
        &mut self,
        plugin: SignedPluginManifest,
        trusted_key: &VerifyingKey,
        granted: &PluginCapabilities,
    ) -> Result<(), ExtensionError> {
        if plugin.public_key != trusted_key.to_bytes() {
            return Err(ExtensionError::Signature);
        }
        trusted_key
            .verify_strict(
                &plugin.manifest.canonical()?,
                &Signature::from_bytes(&plugin.signature),
            )
            .map_err(|_| ExtensionError::Signature)?;
        if !plugin.manifest.capabilities.is_subset_of(granted) {
            return Err(ExtensionError::CapabilityDenied);
        }
        if self.plugins.contains_key(&plugin.manifest.id) {
            return Err(ExtensionError::Duplicate);
        }
        self.plugins.insert(plugin.manifest.id.clone(), plugin);
        Ok(())
    }

    pub fn record_data(&mut self, plugin_id: &str, digest: [u8; 32]) -> Result<(), ExtensionError> {
        if !self.plugins.contains_key(plugin_id) {
            return Err(ExtensionError::NotFound);
        }
        self.data
            .entry(plugin_id.to_owned())
            .or_default()
            .push(digest);
        Ok(())
    }

    pub fn remove(&mut self, plugin_id: &str) -> Result<(), ExtensionError> {
        self.plugins
            .remove(plugin_id)
            .map(|_| ())
            .ok_or(ExtensionError::NotFound)
    }

    pub fn data_for(&self, plugin_id: &str) -> &[[u8; 32]] {
        self.data.get(plugin_id).map(Vec::as_slice).unwrap_or(&[])
    }
}

struct WasiState {
    wasi: WasiP1Ctx,
    limits: StoreLimits,
}

pub struct WasiPluginHost {
    engine: Engine,
    capabilities: PluginCapabilities,
    fuel: u64,
    maximum_memory_bytes: usize,
}

impl WasiPluginHost {
    pub fn new(
        capabilities: PluginCapabilities,
        fuel: u64,
        maximum_memory_bytes: usize,
    ) -> Result<Self, ExtensionError> {
        if fuel == 0 || maximum_memory_bytes == 0 || !capabilities.is_empty() {
            return Err(ExtensionError::CapabilityDenied);
        }
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine =
            Engine::new(&config).map_err(|error| ExtensionError::Wasi(error.to_string()))?;
        Ok(Self {
            engine,
            capabilities,
            fuel,
            maximum_memory_bytes,
        })
    }

    pub fn execute_i32(&self, module_bytes: &[u8], export: &str) -> Result<i32, ExtensionError> {
        if module_bytes.is_empty()
            || module_bytes.len() > MAX_PLUGIN_BYTES
            || !valid_label(export)
            || !self.capabilities.is_empty()
        {
            return Err(ExtensionError::Invalid);
        }
        let module = Module::new(&self.engine, module_bytes)
            .map_err(|error| ExtensionError::Wasi(error.to_string()))?;
        let mut linker: Linker<WasiState> = Linker::new(&self.engine);
        p1::add_to_linker_sync(&mut linker, |state| &mut state.wasi)
            .map_err(|error| ExtensionError::Wasi(error.to_string()))?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(self.maximum_memory_bytes)
            .instances(1)
            .tables(1)
            .build();
        let mut store = Store::new(
            &self.engine,
            WasiState {
                wasi: WasiCtxBuilder::new().build_p1(),
                limits,
            },
        );
        store.limiter(|state| &mut state.limits);
        store
            .set_fuel(self.fuel)
            .map_err(|error| ExtensionError::Wasi(error.to_string()))?;
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|error| ExtensionError::Wasi(error.to_string()))?;
        instance
            .get_typed_func::<(), i32>(&mut store, export)
            .and_then(|function| function.call(&mut store, ()))
            .map_err(|error| ExtensionError::Wasi(error.to_string()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiPluginBroker {
    allowed_methods: BTreeSet<String>,
}

impl UiPluginBroker {
    pub fn new(allowed_methods: BTreeSet<String>) -> Self {
        Self { allowed_methods }
    }

    pub fn authorize(&self, method: &str) -> Result<(), ExtensionError> {
        if !valid_label(method) || !self.allowed_methods.contains(method) {
            return Err(ExtensionError::CapabilityDenied);
        }
        Ok(())
    }

    pub fn content_security_policy(&self) -> &'static str {
        "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; connect-src 'none'"
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpToolDefinition {
    pub name: String,
    pub schema_digest: [u8; 32],
    pub required_capabilities: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpInvocation {
    pub tool: String,
    pub schema_digest: [u8; 32],
    pub environment: BTreeMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct McpGateway {
    tools: BTreeMap<String, McpToolDefinition>,
    frozen: bool,
    rate_limit: u64,
    calls: u64,
}

impl McpGateway {
    pub fn new(rate_limit: u64) -> Self {
        Self {
            tools: BTreeMap::new(),
            frozen: false,
            rate_limit,
            calls: 0,
        }
    }

    pub fn register(&mut self, tool: McpToolDefinition) -> Result<(), ExtensionError> {
        if self.frozen {
            return Err(ExtensionError::Frozen);
        }
        if !valid_label(&tool.name)
            || tool
                .required_capabilities
                .iter()
                .any(|capability| !valid_capability(capability))
        {
            return Err(ExtensionError::Invalid);
        }
        if self.tools.insert(tool.name.clone(), tool).is_some() {
            return Err(ExtensionError::Duplicate);
        }
        Ok(())
    }

    pub fn freeze(&mut self) {
        self.frozen = true;
    }

    pub fn invoke(
        &mut self,
        tool_name: &str,
        schema_digest: [u8; 32],
        granted_capabilities: &BTreeSet<String>,
        inherited_environment: &BTreeMap<String, String>,
    ) -> Result<McpInvocation, ExtensionError> {
        if !self.frozen {
            return Err(ExtensionError::Frozen);
        }
        let tool = self.tools.get(tool_name).ok_or(ExtensionError::NotFound)?;
        if tool.schema_digest != schema_digest {
            return Err(ExtensionError::SchemaMutation);
        }
        if !tool.required_capabilities.is_subset(granted_capabilities) {
            return Err(ExtensionError::CapabilityDenied);
        }
        if self.calls >= self.rate_limit {
            return Err(ExtensionError::RateLimited);
        }
        let environment = inherited_environment
            .iter()
            .filter(|(key, _)| matches!(key.as_str(), "LANG" | "LC_ALL" | "TZ"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        self.calls = self
            .calls
            .checked_add(1)
            .ok_or(ExtensionError::RateLimited)?;
        Ok(McpInvocation {
            tool: tool_name.to_owned(),
            schema_digest,
            environment,
        })
    }
}
