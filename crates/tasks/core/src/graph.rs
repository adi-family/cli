use crate::error::Result;
use crate::storage::TaskStorage;
use crate::types::TaskId;
use std::collections::{HashMap, HashSet};

/// Detects cycles in the task dependency graph using DFS
pub fn detect_cycles(storage: &dyn TaskStorage) -> Result<Vec<Vec<TaskId>>> {
    let deps = storage.get_all_dependencies()?;

    // Build adjacency list
    let mut graph: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
    let mut all_nodes: HashSet<TaskId> = HashSet::new();

    for (from, to) in deps {
        graph.entry(from).or_default().push(to);
        all_nodes.insert(from);
        all_nodes.insert(to);
    }

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
                // Found a cycle - extract it from path
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

/// Check if adding a dependency would create a cycle
pub fn would_create_cycle(storage: &dyn TaskStorage, from: TaskId, to: TaskId) -> Result<bool> {
    // If there's a path from 'to' to 'from', adding from->to would create a cycle
    let deps = storage.get_all_dependencies()?;

    // Build adjacency list
    let mut graph: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
    for (f, t) in deps {
        graph.entry(f).or_default().push(t);
    }

    // Check if 'from' is reachable from 'to'
    let mut visited: HashSet<TaskId> = HashSet::new();
    let mut stack = vec![to];

    while let Some(current) = stack.pop() {
        if current == from {
            return Ok(true); // Adding from->to would create a cycle
        }

        if visited.insert(current) {
            if let Some(neighbors) = graph.get(&current) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }
    }

    Ok(false)
}

/// Get all tasks that transitively depend on the given task
pub fn get_transitive_dependents(storage: &dyn TaskStorage, id: TaskId) -> Result<Vec<TaskId>> {
    let deps = storage.get_all_dependencies()?;

    // Build reverse adjacency list (to -> from mapping)
    let mut reverse_graph: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
    for (from, to) in deps {
        reverse_graph.entry(to).or_default().push(from);
    }

    let mut result = Vec::new();
    let mut visited: HashSet<TaskId> = HashSet::new();
    let mut stack = vec![id];

    while let Some(current) = stack.pop() {
        if visited.insert(current) {
            if current != id {
                result.push(current);
            }

            if let Some(dependents) = reverse_graph.get(&current) {
                for &dependent in dependents {
                    if !visited.contains(&dependent) {
                        stack.push(dependent);
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Get all tasks that the given task transitively depends on
pub fn get_transitive_dependencies(storage: &dyn TaskStorage, id: TaskId) -> Result<Vec<TaskId>> {
    let deps = storage.get_all_dependencies()?;

    // Build adjacency list
    let mut graph: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
    for (from, to) in deps {
        graph.entry(from).or_default().push(to);
    }

    let mut result = Vec::new();
    let mut visited: HashSet<TaskId> = HashSet::new();
    let mut stack = vec![id];

    while let Some(current) = stack.pop() {
        if visited.insert(current) {
            if current != id {
                result.push(current);
            }

            if let Some(dependencies) = graph.get(&current) {
                for &dependency in dependencies {
                    if !visited.contains(&dependency) {
                        stack.push(dependency);
                    }
                }
            }
        }
    }

    Ok(result)
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
    fn test_would_create_cycle() {
        let (storage, _dir) = setup_storage();

        let t1 = storage.create_task(&Task::new("Task 1")).unwrap();
        let t2 = storage.create_task(&Task::new("Task 2")).unwrap();

        storage.add_dependency(t2, t1).unwrap(); // t2 depends on t1

        // Adding t1->t2 would create a cycle
        assert!(would_create_cycle(&storage, t1, t2).unwrap());

        // Adding t1->new_task wouldn't create a cycle
        let t3 = storage.create_task(&Task::new("Task 3")).unwrap();
        assert!(!would_create_cycle(&storage, t1, t3).unwrap());
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
