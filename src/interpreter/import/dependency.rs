//! Dependency tracking system for circular dependency detection and analysis

use super::types::*;
use std::collections::{HashMap, HashSet, VecDeque};

/// Dependency tracker using graph-based analysis for circular dependency detection
pub struct DependencyTracker {
    /// Directed graph of module dependencies
    /// Key: importing module, Value: set of imported modules with import types
    dependency_graph: HashMap<ModulePath, HashMap<ModulePath, ImportType>>,
    /// Stack of modules currently being loaded (for immediate cycle detection)
    loading_stack: Vec<ModulePath>,
    /// Cache of transitive dependency calculations
    transitive_cache: HashMap<ModulePath, HashSet<ModulePath>>,
}

impl DependencyTracker {
    pub fn new() -> Self {
        Self {
            dependency_graph: HashMap::new(),
            loading_stack: Vec::new(),
            transitive_cache: HashMap::new(),
        }
    }

    /// Add an import edge to the dependency graph
    pub fn add_import_edge(&mut self, from: ModulePath, to: ModulePath, import_type: ImportType) {
        self.dependency_graph
            .entry(from)
            .or_insert_with(HashMap::new)
            .insert(to, import_type);

        // Invalidate transitive cache since graph has changed
        self.transitive_cache.clear();
    }

    /// Check for circular dependency before adding a new import
    pub fn check_circular_dependency(
        &self,
        from: ModulePath,
        to: ModulePath,
    ) -> Result<(), CircularDependencyError> {
        // Quick check: if target module depends on source module, it's circular
        if self.has_path(&to, &from) {
            let cycle = self.find_cycle(&from, &to)?;
            let import_chain = self.build_import_chain(&cycle);
            return Err(CircularDependencyError {
                cycle,
                import_chain,
            });
        }

        // Check if the new dependency would create a cycle
        if self.would_create_cycle(&from, &to) {
            let cycle = self.find_cycle(&from, &to)?;
            let import_chain = self.build_import_chain(&cycle);
            return Err(CircularDependencyError {
                cycle,
                import_chain,
            });
        }

        Ok(())
    }

    /// Push a module onto the loading stack
    pub fn push_loading(&mut self, module: ModulePath) -> Result<(), CircularDependencyError> {
        if self.loading_stack.contains(&module) {
            // Found immediate circular dependency in loading stack
            let cycle_start = self
                .loading_stack
                .iter()
                .position(|m| m == &module)
                .unwrap();
            let cycle = self.loading_stack[cycle_start..].to_vec();
            let import_chain = vec![ImportType::Glob; cycle.len()]; // Default to glob

            return Err(CircularDependencyError {
                cycle,
                import_chain,
            });
        }

        self.loading_stack.push(module);
        Ok(())
    }

    /// Pop a module from the loading stack
    pub fn pop_loading(&mut self) -> Option<ModulePath> {
        self.loading_stack.pop()
    }

    /// Get all transitive dependencies of a module
    pub fn get_transitive_dependencies(&mut self, module: &ModulePath) -> HashSet<ModulePath> {
        // Check cache first
        if let Some(cached) = self.transitive_cache.get(module) {
            return cached.clone();
        }

        // Calculate transitive dependencies using DFS
        let mut visited = HashSet::new();
        let mut result = HashSet::new();
        self.dfs_transitive(module, &mut visited, &mut result);

        // Cache the result
        self.transitive_cache.insert(module.clone(), result.clone());

        result
    }

    /// Get direct dependencies of a module
    pub fn get_direct_dependencies(&self, module: &ModulePath) -> Vec<(ModulePath, ImportType)> {
        self.dependency_graph
            .get(module)
            .map(|deps| {
                deps.iter()
                    .map(|(path, import_type)| (path.clone(), import_type.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all modules that depend on the given module
    pub fn get_dependents(&self, module: &ModulePath) -> Vec<ModulePath> {
        let mut dependents = Vec::new();

        for (importing_module, dependencies) in &self.dependency_graph {
            if dependencies.contains_key(module) {
                dependents.push(importing_module.clone());
            }
        }

        dependents
    }

    /// Check if there's a path from source to target in the dependency graph
    fn has_path(&self, from: &ModulePath, to: &ModulePath) -> bool {
        if from == to {
            return true;
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.clone());
        visited.insert(from.clone());

        while let Some(current) = queue.pop_front() {
            if let Some(dependencies) = self.dependency_graph.get(&current) {
                for dependency in dependencies.keys() {
                    if dependency == to {
                        return true;
                    }

                    if !visited.contains(dependency) {
                        visited.insert(dependency.clone());
                        queue.push_back(dependency.clone());
                    }
                }
            }
        }

        false
    }

    /// Check if adding an edge would create a cycle
    fn would_create_cycle(&self, from: &ModulePath, to: &ModulePath) -> bool {
        // Temporarily add the edge and check for cycles
        // This is more thorough than just checking has_path

        // If there's already a path from 'to' back to 'from', adding this edge creates a cycle
        self.has_path(to, from)
    }

    /// Find a cycle in the graph starting from the given edge
    fn find_cycle(
        &self,
        from: &ModulePath,
        to: &ModulePath,
    ) -> Result<Vec<ModulePath>, CircularDependencyError> {
        // Use DFS to find a path from 'to' back to 'from'
        let mut path = Vec::new();
        let mut visited = HashSet::new();

        if self.dfs_find_path(to, from, &mut path, &mut visited) {
            // Add the original 'from' to complete the cycle
            path.push(from.clone());
            Ok(path)
        } else {
            // Fallback: return the immediate edge if no longer path found
            Ok(vec![from.clone(), to.clone()])
        }
    }

    /// DFS to find a path between two modules
    fn dfs_find_path(
        &self,
        current: &ModulePath,
        target: &ModulePath,
        path: &mut Vec<ModulePath>,
        visited: &mut HashSet<ModulePath>,
    ) -> bool {
        path.push(current.clone());
        visited.insert(current.clone());

        if current == target {
            return true;
        }

        if let Some(dependencies) = self.dependency_graph.get(current) {
            for dependency in dependencies.keys() {
                if !visited.contains(dependency) {
                    if self.dfs_find_path(dependency, target, path, visited) {
                        return true;
                    }
                }
            }
        }

        path.pop();
        false
    }

    /// Build import chain from cycle path
    fn build_import_chain(&self, cycle: &[ModulePath]) -> Vec<ImportType> {
        let mut import_chain = Vec::new();

        for i in 0..cycle.len() {
            let from = &cycle[i];
            let to = &cycle[(i + 1) % cycle.len()];

            if let Some(dependencies) = self.dependency_graph.get(from) {
                if let Some(import_type) = dependencies.get(to) {
                    import_chain.push(import_type.clone());
                } else {
                    // Fallback if not found
                    import_chain.push(ImportType::Glob);
                }
            } else {
                import_chain.push(ImportType::Glob);
            }
        }

        import_chain
    }

    /// DFS helper for transitive dependency calculation
    fn dfs_transitive(
        &self,
        current: &ModulePath,
        visited: &mut HashSet<ModulePath>,
        result: &mut HashSet<ModulePath>,
    ) {
        if visited.contains(current) {
            return;
        }

        visited.insert(current.clone());

        if let Some(dependencies) = self.dependency_graph.get(current) {
            for dependency in dependencies.keys() {
                result.insert(dependency.clone());
                self.dfs_transitive(dependency, visited, result);
            }
        }
    }

    /// Clear all dependency tracking data
    pub fn clear(&mut self) {
        self.dependency_graph.clear();
        self.loading_stack.clear();
        self.transitive_cache.clear();
    }

    /// Get dependency graph statistics for debugging
    pub fn get_stats(&self) -> DependencyStats {
        let total_modules = self.dependency_graph.len();
        let total_edges: usize = self.dependency_graph.values().map(|deps| deps.len()).sum();

        let mut import_type_counts = HashMap::new();
        for dependencies in self.dependency_graph.values() {
            for import_type in dependencies.values() {
                *import_type_counts.entry(import_type.clone()).or_insert(0) += 1;
            }
        }

        DependencyStats {
            total_modules,
            total_edges,
            loading_stack_depth: self.loading_stack.len(),
            import_type_counts,
            cache_size: self.transitive_cache.len(),
        }
    }
}

/// Statistics about the dependency graph
#[derive(Debug, Clone)]
pub struct DependencyStats {
    pub total_modules: usize,
    pub total_edges: usize,
    pub loading_stack_depth: usize,
    pub import_type_counts: HashMap<ImportType, usize>,
    pub cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_dependency() {
        let mut tracker = DependencyTracker::new();
        let module_a = ModulePath::from_string("a");
        let module_b = ModulePath::from_string("b");

        // Add A -> B dependency
        tracker.add_import_edge(module_a.clone(), module_b.clone(), ImportType::Glob);

        // Should not be circular
        let result = tracker.check_circular_dependency(module_a.clone(), module_b.clone());
        assert!(result.is_ok());

        // Check direct dependencies
        let deps = tracker.get_direct_dependencies(&module_a);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].0, module_b);
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut tracker = DependencyTracker::new();
        let module_a = ModulePath::from_string("a");
        let module_b = ModulePath::from_string("b");

        // Add A -> B
        tracker.add_import_edge(module_a.clone(), module_b.clone(), ImportType::Glob);

        // Try to add B -> A (should detect cycle)
        let result = tracker.check_circular_dependency(module_b.clone(), module_a.clone());
        assert!(result.is_err());

        if let Err(cycle_error) = result {
            assert!(cycle_error.cycle.contains(&module_a));
            assert!(cycle_error.cycle.contains(&module_b));
        }
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut tracker = DependencyTracker::new();
        let module_a = ModulePath::from_string("a");
        let module_b = ModulePath::from_string("b");
        let module_c = ModulePath::from_string("c");

        // Add A -> B -> C
        tracker.add_import_edge(module_a.clone(), module_b.clone(), ImportType::Glob);
        tracker.add_import_edge(
            module_b.clone(),
            module_c.clone(),
            ImportType::Simple("test".to_string()),
        );

        // A should transitively depend on both B and C
        let transitive = tracker.get_transitive_dependencies(&module_a);
        assert!(transitive.contains(&module_b));
        assert!(transitive.contains(&module_c));
        assert_eq!(transitive.len(), 2);
    }

    #[test]
    fn test_loading_stack_cycle_detection() {
        let mut tracker = DependencyTracker::new();
        let module_a = ModulePath::from_string("a");
        let module_b = ModulePath::from_string("b");

        // Push A, then B
        assert!(tracker.push_loading(module_a.clone()).is_ok());
        assert!(tracker.push_loading(module_b.clone()).is_ok());

        // Try to push A again (should detect immediate cycle)
        let result = tracker.push_loading(module_a.clone());
        assert!(result.is_err());
    }

    #[test]
    fn test_complex_cycle_detection() {
        let mut tracker = DependencyTracker::new();
        let module_a = ModulePath::from_string("a");
        let module_b = ModulePath::from_string("b");
        let module_c = ModulePath::from_string("c");
        let module_d = ModulePath::from_string("d");

        // Create A -> B -> C -> D
        tracker.add_import_edge(module_a.clone(), module_b.clone(), ImportType::Glob);
        tracker.add_import_edge(
            module_b.clone(),
            module_c.clone(),
            ImportType::Selective(vec!["test".to_string()]),
        );
        tracker.add_import_edge(
            module_c.clone(),
            module_d.clone(),
            ImportType::Simple("test".to_string()),
        );

        // Try to add D -> A (should detect long cycle)
        let result = tracker.check_circular_dependency(module_d.clone(), module_a.clone());
        assert!(result.is_err());

        if let Err(cycle_error) = result {
            // Should find the complete cycle
            assert!(cycle_error.cycle.len() >= 4);
        }
    }
}
