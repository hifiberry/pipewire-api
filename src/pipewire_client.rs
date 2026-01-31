use anyhow::{anyhow, Result};
use pipewire as pw;
use regex::Regex;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Information about a discovered PipeWire node
#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub id: u32,
    pub name: String,
}

/// PipeWire client for managing connections and finding nodes
pub struct PipeWireClient {
    mainloop: pw::main_loop::MainLoopRc,
    // These fields must be kept alive even though they're not directly accessed.
    // The registry depends on core, which depends on context, which depends on mainloop.
    // Dropping these would cause the underlying PipeWire objects to be destroyed.
    #[allow(dead_code)]
    context: pw::context::ContextRc,
    #[allow(dead_code)]
    core: pw::core::CoreRc,
    registry: pw::registry::RegistryRc,
}

impl PipeWireClient {
    /// Create a new PipeWire client
    pub fn new() -> Result<Self> {
        pw::init();

        let mainloop = pw::main_loop::MainLoopRc::new(None)?;
        let context = pw::context::ContextRc::new(&mainloop, None)?;
        let core = context.connect_rc(None)?;
        let registry = core.get_registry_rc()?;

        Ok(Self {
            mainloop,
            context,
            core,
            registry,
        })
    }

    /// Get the mainloop reference
    pub fn mainloop(&self) -> &pw::main_loop::MainLoopRc {
        &self.mainloop
    }

    /// Get the registry reference
    pub fn registry(&self) -> &pw::registry::RegistryRc {
        &self.registry
    }

    /// Find a specific node by name with timeout
    pub fn find_node(&self, node_name: &str, timeout_secs: u64) -> Result<NodeInfo> {
        let node_info: Rc<RefCell<Option<NodeInfo>>> = Rc::new(RefCell::new(None));
        let node_info_clone = node_info.clone();
        
        let done = Rc::new(Cell::new(false));
        let done_for_closure = done.clone();
        let mainloop_clone = self.mainloop.clone();
        
        // Set up timeout timer
        let timeout_done = done.clone();
        let timeout_mainloop = self.mainloop.clone();
        let _timer = self.mainloop.loop_().add_timer(move |_| {
            if !timeout_done.get() {
                timeout_mainloop.quit();
            }
        });
        _timer.update_timer(
            Some(std::time::Duration::from_secs(timeout_secs)),
            None
        );
        
        let registry_weak = self.registry.downgrade();
        let target_name = node_name.to_string();
        
        // Find node by name
        let _listener = self.registry
            .add_listener_local()
            .global({
                move |global| {
                    if let Some(_registry) = registry_weak.upgrade() {
                        if let Some(props) = &global.props {
                            if global.type_ == pw::types::ObjectType::Node {
                                if let Some(name) = props.get("node.name") {
                                    if name == target_name {
                                        *node_info_clone.borrow_mut() = Some(NodeInfo {
                                            id: global.id,
                                            name: name.to_string(),
                                        });
                                        
                                        done_for_closure.set(true);
                                        mainloop_clone.quit();
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .register();

        // Run until we find the node or timeout
        self.mainloop.run();
        
        if !done.get() {
            return Err(anyhow!("Timeout: Node '{}' not found", node_name));
        }

        let result = node_info.borrow().clone();
        result.ok_or_else(|| anyhow!("Node '{}' not found", node_name))
    }

    /// Find and bind a node by name
    pub fn find_and_bind_node(&self, node_name: &str, timeout_secs: u64) -> Result<(NodeInfo, pw::node::Node)> {
        let node_info: Rc<RefCell<Option<NodeInfo>>> = Rc::new(RefCell::new(None));
        let node_info_clone = node_info.clone();
        
        let node_obj: Rc<RefCell<Option<pw::node::Node>>> = Rc::new(RefCell::new(None));
        let node_obj_clone = node_obj.clone();
        
        let done = Rc::new(Cell::new(false));
        let done_for_closure = done.clone();
        let mainloop_clone = self.mainloop.clone();
        
        // Set up timeout timer
        let timeout_done = done.clone();
        let timeout_mainloop = self.mainloop.clone();
        let _timer = self.mainloop.loop_().add_timer(move |_| {
            if !timeout_done.get() {
                timeout_mainloop.quit();
            }
        });
        _timer.update_timer(
            Some(std::time::Duration::from_secs(timeout_secs)),
            None
        );
        
        let registry_weak = self.registry.downgrade();
        let target_name = node_name.to_string();
        
        // Find and bind node
        let _listener = self.registry
            .add_listener_local()
            .global({
                move |global| {
                    if let Some(registry) = registry_weak.upgrade() {
                        if let Some(props) = &global.props {
                            if global.type_ == pw::types::ObjectType::Node {
                                if let Some(name) = props.get("node.name") {
                                    if name == target_name {
                                        *node_info_clone.borrow_mut() = Some(NodeInfo {
                                            id: global.id,
                                            name: name.to_string(),
                                        });
                                        
                                        // Bind the node
                                        if let Ok(n) = registry.bind::<pw::node::Node, _>(&global) {
                                            *node_obj_clone.borrow_mut() = Some(n);
                                        }
                                        
                                        done_for_closure.set(true);
                                        mainloop_clone.quit();
                                    }
                                }
                            }
                        }
                    }
                }
            })
            .register();

        // Run until we find the node or timeout
        self.mainloop.run();
        
        if !done.get() {
            return Err(anyhow!("Timeout: Node '{}' not found", node_name));
        }

        let info = node_info.borrow().clone()
            .ok_or_else(|| anyhow!("Node '{}' not found", node_name))?;
        
        let node = node_obj.borrow_mut().take()
            .ok_or_else(|| anyhow!("Failed to bind node '{}'", node_name))?;

        Ok((info, node))
    }

    /// Find all nodes matching a regex pattern
    pub fn find_nodes_by_pattern(&self, pattern: &str, timeout_secs: u64) -> Result<Vec<NodeInfo>> {
        let regex = Regex::new(pattern)?;
        let found_nodes: Rc<RefCell<Vec<NodeInfo>>> = Rc::new(RefCell::new(Vec::new()));
        let found_nodes_clone = found_nodes.clone();
        
        let done = Rc::new(Cell::new(false));
        let done_for_remove = done.clone();
        let mainloop_for_remove = self.mainloop.clone();
        
        // Set up timeout timer
        let timeout_done = done.clone();
        let timeout_mainloop = self.mainloop.clone();
        let _timer = self.mainloop.loop_().add_timer(move |_| {
            timeout_done.set(true);
            timeout_mainloop.quit();
        });
        _timer.update_timer(
            Some(std::time::Duration::from_secs(timeout_secs)),
            None
        );
        
        // Listen for all nodes
        let _listener = self.registry
            .add_listener_local()
            .global({
                let regex = regex.clone();
                move |global| {
                    if global.type_ == pw::types::ObjectType::Node {
                        if let Some(props) = &global.props {
                            if let Some(name) = props.get("node.name") {
                                if regex.is_match(name) {
                                    found_nodes_clone.borrow_mut().push(NodeInfo {
                                        id: global.id,
                                        name: name.to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            })
            .global_remove({
                move |_id| {
                    // Registry enumeration is complete
                    done_for_remove.set(true);
                    mainloop_for_remove.quit();
                }
            })
            .register();

        // Run mainloop until timeout or completion
        self.mainloop.run();
        
        let result = found_nodes.borrow().clone();
        Ok(result)
    }
}
