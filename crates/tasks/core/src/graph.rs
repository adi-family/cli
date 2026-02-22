//! Graph algorithms for task dependency management.
//!
//! Circular dependencies are allowed. This module provides algorithms for:
//! - Cycle detection in dependency graphs (reporting, not prevention)
//! - Computing transitive dependencies and dependents

use crate::error::Result;
use crate::storage::TaskStorage;
use crate::types::TaskId;
use std::collections::{HashMap, HashSet};

fn build_adjacency_list(deps: &[(TaskId, TaskId)]) -> HashMap<TaskId, Vec<TaskId>> {
    let mut graph: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
    for &(from, to) in deps {
        graph.entry(from).or_default().push(to);
    }
    graph
}

fn build_reverse_adjacency_list(deps: &[(TaskId, TaskId)]) -> HashMap<TaskId, Vec<TaskId>> {
    let mut graph: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
    for &(from, to) in deps {
        graph.entry(to).or_default().push(from);
    }
    graph
}

fn collect_all_nodes(deps: &[(TaskId, TaskId)]) -> HashSet<TaskId> {
    let mut nodes = HashSet::new();
    for &(from, to) in deps {
        nodes.insert(from);
        nodes.insert(to);
    }
    nodes
}

/// Returns a list of cycles, where each cycle is a list of TaskIds forming a circular dependency.
pub fn detect_cycles(storage: &dyn TaskStorage) -> Result<Vec<Vec<TaskId>>> {
    let deps = storage.get_all_dependencies()?;
    let graph = build_adjacency_list(&deps);
    let all_nodes = collect_all_nodes(&deps);

    let mut cycles = Vec::new();
    let mut visited: HashSet<TaskId> = HashSet::new();
    let mut rec_stack: HashSet<TaskId> = HashSet::new();
    let mut path: Vec<TaskId> = Vec::new();

    for &node in &all_nodes {
        if !visited.contains(&node) {
            dfs_detect_cycle(
                node,
                &graph,
                &mut visited,
                &mut rec_stack,
                &mut path,
                &mut cycles,
            );
        }
    }

    Ok(cycles)
}

fn dfs_detect_cycle(
    node: TaskId,
    graph: &HashMap<TaskId, Vec<TaskId>>,
    visited: &mut HashSet<TaskId>,
    rec_stack: &mut HashSet<TaskId>,
    path: &mut Vec<TaskId>,
    cycles: &mut Vec<Vec<TaskId>>,
) {
    visited.insert(node);
    rec_stack.insert(node);
    path.push(node);

    if let Some(neighbors) = graph.get(&node) {
        for &neighbor in neighbors {
            if !visited.contains(&neighbor) {
                dfs_detect_cycle(neighbor, graph, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(&neighbor) {
                if let Some(start) = path.iter().position(|&n| n == neighbor) {
                    let cycle: Vec<TaskId> = path[start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }
    }

    path.pop();
    rec_stack.remove(&node);
}

/// Includes both direct dependents and their dependents, recursively.
pub fn get_transitive_dependents(storage: &dyn TaskStorage, id: TaskId) -> Result<Vec<TaskId>> {
    let deps = storage.get_all_dependencies()?;
    let reverse_graph = build_reverse_adjacency_list(&deps);
    Ok(collect_reachable(&reverse_graph, id))
}

/// Includes both direct dependencies and their dependencies, recursively.
pub fn get_transitive_dependencies(storage: &dyn TaskStorage, id: TaskId) -> Result<Vec<TaskId>> {
    let deps = storage.get_all_dependencies()?;
    let graph = build_adjacency_list(&deps);
    Ok(collect_reachable(&graph, id))
}

fn collect_reachable(graph: &HashMap<TaskId, Vec<TaskId>>, start: TaskId) -> Vec<TaskId> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut stack = vec![start];

    while let Some(current) = stack.pop() {
        if visited.insert(current) {
            if current != start {
                result.push(current);
            }

            if let Some(neighbors) = graph.get(&current) {
                stack.extend(neighbors.iter().filter(|n| !visited.contains(n)));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::SqliteTaskStorage;
    use crate::types::Task;
    use tempfile::tempdir;

    fn setup_storage() -> (SqliteTaskStorage, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("tasks.sqlite");
        let storage = SqliteTaskStorage::open(&db_path).unwrap();
        (storage, dir)
    }

    #[test]
    fn test_no_cycles() {
        let (storage, _dir) = setup_storage();

        let t1 = storage.create_task(&Task::new("Task 1")).unwrap();
        let t2 = storage.create_task(&Task::new("Task 2")).unwrap();
        let t3 = storage.create_task(&Task::new("Task 3")).unwrap();

        // t3 -> t2 -> t1 (linear chain)
        storage.add_dependency(t3, t2).unwrap();
        storage.add_dependency(t2, t1).unwrap();

        let cycles = detect_cycles(&storage).unwrap();
        assert!(cycles.is_empty());
    }

    #[test]
    fn test_detect_cycle() {
        let (storage, _dir) = setup_storage();

        let t1 = storage.create_task(&Task::new("Task 1")).unwrap();
        let t2 = storage.create_task(&Task::new("Task 2")).unwrap();
        let t3 = storage.create_task(&Task::new("Task 3")).unwrap();

        // Create a cycle: t1 -> t2 -> t3 -> t1
        storage.add_dependency(t1, t2).unwrap();
        storage.add_dependency(t2, t3).unwrap();
        storage.add_dependency(t3, t1).unwrap();

        let cycles = detect_cycles(&storage).unwrap();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_transitive_dependencies() {
        let (storage, _dir) = setup_storage();

        let t1 = storage.create_task(&Task::new("Task 1")).unwrap();
        let t2 = storage.create_task(&Task::new("Task 2")).unwrap();
        let t3 = storage.create_task(&Task::new("Task 3")).unwrap();

        // t3 -> t2 -> t1
        storage.add_dependency(t3, t2).unwrap();
        storage.add_dependency(t2, t1).unwrap();

        let deps = get_transitive_dependencies(&storage, t3).unwrap();
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&t1));
        assert!(deps.contains(&t2));
    }
}
