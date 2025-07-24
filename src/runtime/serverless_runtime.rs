//! Serverless runtime integration for REAM
//! 
//! Integrates hibernation manager, cold start optimizer, resource pools,
//! and serverless features with the main REAM runtime system.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::types::Pid;
use crate::error::RuntimeResult;
use crate::runtime::serverless::*;

/// Serverless-enabled REAM runtime
pub struct ServerlessReamRuntime {
    /// Core hibernation manager
    hibernation_manager: Arc<HibernationManager>,
    /// Cold start optimizer
    cold_start_optimizer: Arc<ColdStartOptimizer>,
    /// Resource pools
    resource_pools: Arc<ResourcePools>,
    /// Zero-copy hibernation system
    zero_copy_system: Arc<ZeroCopyHibernation>,
    /// Metrics collector
    metrics: Arc<ServerlessMetrics>,
    /// Serverless configuration
    config: ServerlessConfig,
    /// Active serverless functions
    functions: Arc<RwLock<HashMap<String, ServerlessFunction>>>,
    /// Active deployments
    deployments: Arc<RwLock<HashMap<String, ServerlessDeployment>>>,
}

impl ServerlessReamRuntime {
    /// Create a new serverless runtime
    pub fn new(config: ServerlessConfig) -> RuntimeResult<Self> {
        // Initialize hibernation manager
        let hibernation_manager = Arc::new(HibernationManager::new());
        
        // Initialize cold start optimizer
        let cold_start_config = ColdStartConfig {
            pre_warming_enabled: true,
            pre_warm_sizes: config.pre_warm_pools.clone(),
            bytecode_cache_enabled: config.jit_cache_enabled,
            jit_cache_enabled: config.jit_cache_enabled,
            ..Default::default()
        };
        let cold_start_optimizer = Arc::new(
            ColdStartOptimizer::new(cold_start_config)
                .map_err(|e| crate::error::RuntimeError::Serverless(format!("Cold start optimizer: {}", e)))?
        );
        
        // Initialize resource pools
        let resource_config = ResourcePoolConfig {
            pre_warming_enabled: true,
            ..Default::default()
        };
        let resource_pools = Arc::new(ResourcePools::new(resource_config));
        
        // Initialize zero-copy system
        let zero_copy_config = ZeroCopyConfig {
            enabled: config.zero_copy_enabled,
            storage_size: config.hibernation_storage_size,
            ..Default::default()
        };
        let zero_copy_system = Arc::new(
            ZeroCopyHibernation::new(zero_copy_config)
                .map_err(|e| crate::error::RuntimeError::Serverless(format!("Zero-copy system: {}", e)))?
        );
        
        // Initialize metrics
        let metrics_config = MetricsConfig::default();
        let metrics = Arc::new(ServerlessMetrics::new(metrics_config));
        
        Ok(ServerlessReamRuntime {
            hibernation_manager,
            cold_start_optimizer,
            resource_pools,
            zero_copy_system,
            metrics,
            config,
            functions: Arc::new(RwLock::new(HashMap::new())),
            deployments: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Deploy a serverless function
    pub fn deploy_function(&self, function: ServerlessFunction) -> RuntimeResult<()> {
        let function_name = function.name.clone();
        let actor_type = function.actor_type.clone();
        
        // Pre-warm resources for this function
        self.cold_start_optimizer.pre_warm(&actor_type)
            .map_err(|e| crate::error::RuntimeError::Serverless(format!("Pre-warming failed: {}", e)))?;
        
        self.resource_pools.pre_warm(&actor_type)
            .map_err(|e| crate::error::RuntimeError::Serverless(format!("Resource pre-warming failed: {}", e)))?;
        
        // Store function
        self.functions.write().unwrap().insert(function_name.clone(), function);
        
        println!("Deployed serverless function: {}", function_name);
        Ok(())
    }
    
    /// Undeploy a serverless function
    pub fn undeploy_function(&self, name: &str) -> RuntimeResult<()> {
        self.functions.write().unwrap().remove(name);
        println!("Undeployed serverless function: {}", name);
        Ok(())
    }
    
    /// Invoke a serverless function
    pub async fn invoke_function(&self, name: &str, payload: Vec<u8>) -> RuntimeResult<Vec<u8>> {
        let start = Instant::now();
        
        // Get function definition
        let function = {
            let functions = self.functions.read().unwrap();
            functions.get(name).cloned()
                .ok_or_else(|| crate::error::RuntimeError::Serverless(format!("Function not found: {}", name)))?
        };
        
        // Check if we have hibernated instances
        let hibernating_actors = self.hibernation_manager.list_hibernating_actors();
        
        let result = if hibernating_actors.is_empty() {
            // Cold start - create new actor
            self.cold_start_invoke(&function, payload).await?
        } else {
            // Warm start - wake hibernated actor
            self.warm_start_invoke(&function, payload).await?
        };
        
        let execution_time = start.elapsed();
        
        // Record metrics
        self.metrics.record_function_invocation(name, execution_time, true);
        
        Ok(result)
    }
    
    /// Cold start function invocation
    async fn cold_start_invoke(&self, function: &ServerlessFunction, payload: Vec<u8>) -> RuntimeResult<Vec<u8>> {
        let pid = Pid::new();
        
        // Perform instant wake with pre-warmed resources
        let wake_time = self.cold_start_optimizer.instant_wake(pid, &function.actor_type)
            .map_err(|e| crate::error::RuntimeError::Serverless(format!("Cold start failed: {}", e)))?;
        
        println!("Cold start completed in {:?}", wake_time);
        
        // Simulate function execution
        let result = self.execute_function_logic(function, payload).await?;
        
        // Hibernate the actor after execution
        self.hibernation_manager.hibernate_actor(pid, function.actor_type.clone(), 1024 * 1024).await
            .map_err(|e| crate::error::RuntimeError::Serverless(format!("Hibernation failed: {}", e)))?;
        
        Ok(result)
    }
    
    /// Warm start function invocation
    async fn warm_start_invoke(&self, function: &ServerlessFunction, payload: Vec<u8>) -> RuntimeResult<Vec<u8>> {
        let hibernating_actors = self.hibernation_manager.list_hibernating_actors();
        let pid = hibernating_actors[0]; // Use first available hibernated actor
        
        // Wake the actor
        let wake_trigger = WakeTrigger::IncomingMessage;
        let _actor_type = self.hibernation_manager.wake_actor(pid, wake_trigger).await
            .map_err(|e| crate::error::RuntimeError::Serverless(format!("Wake failed: {}", e)))?;
        
        println!("Warm start completed for actor {:?}", pid);
        
        // Execute function
        let result = self.execute_function_logic(function, payload).await?;
        
        // Hibernate again
        self.hibernation_manager.hibernate_actor(pid, function.actor_type.clone(), 1024 * 1024).await
            .map_err(|e| crate::error::RuntimeError::Serverless(format!("Re-hibernation failed: {}", e)))?;
        
        Ok(result)
    }
    
    /// Execute function logic (simplified)
    async fn execute_function_logic(&self, function: &ServerlessFunction, payload: Vec<u8>) -> RuntimeResult<Vec<u8>> {
        // Simulate function execution time
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        // Simple echo response for demonstration
        let response = format!("Function {} processed {} bytes", function.name, payload.len());
        Ok(response.into_bytes())
    }
    
    /// Get function metrics
    pub fn get_function_metrics(&self, name: &str) -> Option<crate::runtime::serverless::metrics::FunctionMetrics> {
        self.metrics.get_function_metrics(name)
    }
    
    /// List deployed functions
    pub fn list_functions(&self) -> Vec<String> {
        self.functions.read().unwrap().keys().cloned().collect()
    }
    
    /// Scale function instances
    pub fn scale_function(&self, name: &str, instances: usize) -> RuntimeResult<()> {
        // In a real implementation, this would manage the number of pre-warmed instances
        println!("Scaling function {} to {} instances", name, instances);
        Ok(())
    }
    
    /// Get hibernation statistics
    pub fn get_hibernation_stats(&self) -> HibernationStats {
        self.hibernation_manager.get_stats()
    }
    
    /// Get cold start statistics
    pub fn get_cold_start_stats(&self) -> ColdStartStats {
        self.cold_start_optimizer.get_stats()
    }
    
    /// Get resource statistics
    pub fn get_resource_stats(&self) -> ResourceStats {
        self.resource_pools.get_stats()
    }
    
    /// Get zero-copy statistics
    pub fn get_zero_copy_stats(&self) -> ZeroCopyStats {
        self.zero_copy_system.get_stats()
    }
    
    /// Export all metrics
    pub fn export_metrics(&self) -> Vec<(String, String, String)> {
        // Update metrics from subsystems
        self.metrics.update_hibernation_metrics(self.get_hibernation_stats());
        self.metrics.update_cold_start_metrics(self.get_cold_start_stats());
        self.metrics.update_resource_metrics(self.get_resource_stats());
        
        // Export all formats
        self.metrics.export_all()
    }
    
    /// Deploy a complete serverless deployment
    pub fn deploy_deployment(&self, deployment: ServerlessDeployment) -> RuntimeResult<()> {
        let deployment_name = deployment.name.clone();
        
        // Deploy all functions in the deployment
        for function in &deployment.functions {
            // Deploy the function directly
            let serverless_function = ServerlessFunction {
                name: function.name.clone(),
                actor_type: function.actor_type.clone(),
                memory_limit: 128 * 1024 * 1024, // 128MB
                timeout: Duration::from_secs(30),
                concurrency: 100,
                wake_triggers: vec![WakeTrigger::IncomingMessage],
                environment: HashMap::new(),
            };
            
            self.deploy_function(serverless_function)?;
        }
        
        // Store deployment
        self.deployments.write().unwrap().insert(deployment_name.clone(), deployment);
        
        println!("Deployed serverless deployment: {}", deployment_name);
        Ok(())
    }
    
    /// Get deployment information
    pub fn get_deployment(&self, name: &str) -> Option<ServerlessDeployment> {
        self.deployments.read().unwrap().get(name).cloned()
    }
    
    /// List all deployments
    pub fn list_deployments(&self) -> Vec<String> {
        self.deployments.read().unwrap().keys().cloned().collect()
    }
}

impl ServerlessRuntime for ServerlessReamRuntime {
    fn deploy_function(&mut self, function: ServerlessFunction) -> ServerlessResult<()> {
        ServerlessReamRuntime::deploy_function(self, function)
            .map_err(|e| ServerlessError::Deployment(format!("{}", e)))
    }

    fn undeploy_function(&mut self, name: &str) -> ServerlessResult<()> {
        ServerlessReamRuntime::undeploy_function(self, name)
            .map_err(|e| ServerlessError::Deployment(format!("{}", e)))
    }

    fn invoke_function(&mut self, name: &str, _payload: Vec<u8>) -> ServerlessResult<Vec<u8>> {
        // This is a synchronous interface, so we can't use async here
        // In a real implementation, this would use a runtime executor
        Err(ServerlessError::Deployment("Async invocation not supported in sync interface".to_string()))
    }

    fn get_function_metrics(&self, name: &str) -> ServerlessResult<FunctionMetrics> {
        ServerlessReamRuntime::get_function_metrics(self, name)
            .map(|m| FunctionMetrics {
                name: m.name.clone(),
                invocations: m.invocations,
                avg_execution_time: m.average_execution_time(),
                avg_cold_start_time: m.average_cold_start_time(),
                active_instances: m.active_instances as usize,
                hibernated_instances: m.hibernated_instances as usize,
                memory_usage: m.memory_usage as usize,
                error_rate: m.error_rate,
            })
            .ok_or_else(|| ServerlessError::FunctionNotFound(name.to_string()))
    }

    fn list_functions(&self) -> Vec<String> {
        ServerlessReamRuntime::list_functions(self)
    }

    fn scale_function(&mut self, name: &str, instances: usize) -> ServerlessResult<()> {
        ServerlessReamRuntime::scale_function(self, name, instances)
            .map_err(|e| ServerlessError::Scaling(format!("{}", e)))
    }
}
