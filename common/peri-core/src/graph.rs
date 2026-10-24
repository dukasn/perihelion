use petgraph::algo::{has_path_connecting, is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use semver::{Version, VersionReq};
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::error::ResolveError;
use crate::manifest::Manifest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Requires,
    LoadsAfter,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    manifests: BTreeMap<String, Manifest>,
    extra_edges: Vec<(String, String)>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            manifests: BTreeMap::new(),
            extra_edges: Vec::new(),
        }
    }

    pub fn add_manifest(&mut self, manifest: Manifest) {
        self.manifests
            .insert(manifest.package.name.clone(), manifest);
    }

    pub fn add_edge_by_name(&mut self, from: &str, to: &str) {
        self.extra_edges.push((from.to_string(), to.to_string()));
    }

    pub fn get_manifest(&self, name: &str) -> Option<&Manifest> {
        self.manifests.get(name)
    }

    pub fn build_petgraph(&self) -> (DiGraph<String, EdgeType>, HashMap<String, NodeIndex>) {
        let mut graph = DiGraph::<String, EdgeType>::new();
        let mut node_indices: HashMap<String, NodeIndex> = HashMap::new();

        for name in self.manifests.keys() {
            let idx = graph.add_node(name.clone());
            node_indices.insert(name.clone(), idx);
        }

        // Direct edges: if B depends on A (or B loads after A), edge is A -> B
        // (A must be loaded before B).
        for (name, manifest) in &self.manifests {
            let target_node = node_indices[name];

            for dep_name in manifest.dependencies.keys() {
                if let Some(&dep_node) = node_indices.get(dep_name) {
                    graph.add_edge(dep_node, target_node, EdgeType::Requires);
                }
            }

            if let Some(rules) = &manifest.load_order {
                for after_name in &rules.loads_after {
                    if let Some(&after_node) = node_indices.get(after_name) {
                        graph.add_edge(after_node, target_node, EdgeType::LoadsAfter);
                    }
                }
            }
        }

        for (from, to) in &self.extra_edges {
            if let (Some(&from_idx), Some(&to_idx)) = (node_indices.get(from), node_indices.get(to))
            {
                graph.add_edge(from_idx, to_idx, EdgeType::LoadsAfter);
            }
        }

        (graph, node_indices)
    }

    pub fn has_path(&self, from: &str, to: &str) -> bool {
        let (graph, node_indices) = self.build_petgraph();
        match (node_indices.get(from), node_indices.get(to)) {
            (Some(&from_idx), Some(&to_idx)) => has_path_connecting(&graph, from_idx, to_idx, None),
            _ => false,
        }
    }

    pub fn validate_incompatibilities(&self) -> Result<(), ResolveError> {
        for (name, manifest) in &self.manifests {
            if let Some(rules) = &manifest.load_order {
                for incomp in &rules.incompatible_with {
                    if self.manifests.contains_key(incomp) {
                        return Err(ResolveError::IncompatiblePackages {
                            package_a: name.clone(),
                            package_b: incomp.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub fn validate_requirements(&self) -> Result<(), ResolveError> {
        for (pkg_name, manifest) in &self.manifests {
            for (dep_name, req_str) in &manifest.dependencies {
                match self.manifests.get(dep_name) {
                    None => {
                        return Err(ResolveError::MissingDependency {
                            package: pkg_name.clone(),
                            dependency: dep_name.clone(),
                            constraint: req_str.clone(),
                        });
                    }
                    Some(dep_manifest) => {
                        let parsed_req = VersionReq::parse(req_str).map_err(|_| {
                            ResolveError::MissingDependency {
                                package: pkg_name.clone(),
                                dependency: dep_name.clone(),
                                constraint: req_str.clone(),
                            }
                        })?;
                        let dep_ver =
                            Version::parse(&dep_manifest.package.version).map_err(|_| {
                                ResolveError::VersionMismatch {
                                    package: dep_name.clone(),
                                    found: dep_manifest.package.version.clone(),
                                    required: req_str.clone(),
                                }
                            })?;

                        if !parsed_req.matches(&dep_ver) {
                            return Err(ResolveError::VersionMismatch {
                                package: dep_name.clone(),
                                found: dep_manifest.package.version.clone(),
                                required: req_str.clone(),
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn resolve_load_order(&self) -> Result<Vec<String>, ResolveError> {
        self.validate_incompatibilities()?;
        self.validate_requirements()?;

        let (graph, _node_indices) = self.build_petgraph();

        if is_cyclic_directed(&graph) {
            let cycle_info = self.find_cycle_path(&graph);
            return Err(ResolveError::CyclicDependencyDetected(cycle_info));
        }

        let sorted_indices = toposort(&graph, None).map_err(|cycle| {
            let node_name = graph[cycle.node_id()].clone();
            ResolveError::CyclicDependencyDetected(node_name)
        })?;

        let sorted_names: Vec<String> = sorted_indices
            .into_iter()
            .map(|idx| graph[idx].clone())
            .collect();

        Ok(sorted_names)
    }

    fn find_cycle_path(&self, graph: &DiGraph<String, EdgeType>) -> String {
        // Simple DFS to pinpoint the first detected cycle
        let mut visited = HashSet::new();
        let mut rec_stack = Vec::new();

        for start in graph.node_indices() {
            if let Some(cycle) = self.dfs_cycle(start, graph, &mut visited, &mut rec_stack) {
                return cycle.join(" -> ");
            }
        }

        "Unknown cycle".to_string()
    }

    fn dfs_cycle(
        &self,
        node: NodeIndex,
        graph: &DiGraph<String, EdgeType>,
        visited: &mut HashSet<NodeIndex>,
        rec_stack: &mut Vec<NodeIndex>,
    ) -> Option<Vec<String>> {
        if let Some(pos) = rec_stack.iter().position(|&n| n == node) {
            let mut cycle = rec_stack[pos..].to_vec();
            cycle.push(node);
            return Some(cycle.into_iter().map(|idx| graph[idx].clone()).collect());
        }

        if visited.contains(&node) {
            return None;
        }

        visited.insert(node);
        rec_stack.push(node);

        for neighbor in graph.neighbors(node) {
            if let Some(cycle) = self.dfs_cycle(neighbor, graph, visited, rec_stack) {
                return Some(cycle);
            }
        }

        rec_stack.pop();
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{LoadOrderRules, PackageMetadata, TargetGame};

    fn make_test_manifest(
        name: &str,
        version: &str,
        deps: Vec<(&str, &str)>,
        loads_after: Vec<&str>,
    ) -> Manifest {
        let mut dependencies = BTreeMap::new();
        for (dep, req) in deps {
            dependencies.insert(dep.to_string(), req.to_string());
        }

        Manifest {
            package: PackageMetadata {
                name: name.to_string(),
                author: "Tester".to_string(),
                version: version.to_string(),
                priority: Default::default(),
                description: None,
            },
            target: TargetGame {
                game_id: "test_game".to_string(),
                engine_version: None,
            },
            assets: None,
            dependencies,
            overrides: BTreeMap::new(),
            load_order: Some(LoadOrderRules {
                loads_after: loads_after.into_iter().map(String::from).collect(),
                incompatible_with: Vec::new(),
            }),
        }
    }

    #[test]
    fn test_topological_sort_linear() {
        let mut dags = DependencyGraph::new();
        // C requires B, B requires A
        dags.add_manifest(make_test_manifest("ModA", "1.0.0", vec![], vec![]));
        dags.add_manifest(make_test_manifest(
            "ModB",
            "1.0.0",
            vec![("ModA", ">=1.0.0")],
            vec![],
        ));
        dags.add_manifest(make_test_manifest(
            "ModC",
            "1.0.0",
            vec![("ModB", ">=1.0.0")],
            vec![],
        ));

        let order = dags
            .resolve_load_order()
            .expect("Linear order should resolve");
        assert_eq!(order, vec!["ModA", "ModB", "ModC"]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut dags = DependencyGraph::new();
        // A -> B -> A
        dags.add_manifest(make_test_manifest(
            "ModA",
            "1.0.0",
            vec![("ModB", ">=1.0.0")],
            vec![],
        ));
        dags.add_manifest(make_test_manifest(
            "ModB",
            "1.0.0",
            vec![("ModA", ">=1.0.0")],
            vec![],
        ));

        let err = dags.resolve_load_order().unwrap_err();
        match err {
            ResolveError::CyclicDependencyDetected(cycle) => {
                assert!(cycle.contains("ModA"));
                assert!(cycle.contains("ModB"));
            }
            _ => panic!("Expected cyclic dependency error, got {err:?}"),
        }
    }

    #[test]
    fn test_incompatibility_detection() {
        let mut dags = DependencyGraph::new();
        let mut mod_a = make_test_manifest("ModA", "1.0.0", vec![], vec![]);
        mod_a.load_order = Some(LoadOrderRules {
            loads_after: Vec::new(),
            incompatible_with: vec!["ModB".to_string()],
        });
        let mod_b = make_test_manifest("ModB", "1.0.0", vec![], vec![]);

        dags.add_manifest(mod_a);
        dags.add_manifest(mod_b);

        let err = dags.resolve_load_order().unwrap_err();
        assert_eq!(
            err,
            ResolveError::IncompatiblePackages {
                package_a: "ModA".to_string(),
                package_b: "ModB".to_string()
            }
        );
    }
}
