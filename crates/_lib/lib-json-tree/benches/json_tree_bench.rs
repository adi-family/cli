//! Criterion benchmarks for lib-json-tree
//!
//! Run with: cargo bench -p lib-json-tree

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use lib_json_tree::{
    count_nodes, filter_nodes, find_all_paths, find_path, flatten_json, flatten_json_iter,
    flatten_json_small, flatten_json_to_depth, get_by_pointer, parse_json, path_to_pointer,
    pointer_to_path, JsonPath, JsonTreeState,
};
use serde_json::{json, Value};

/// Generate a nested JSON object of given depth and width
fn generate_nested_json(depth: usize, width: usize) -> Value {
    if depth == 0 {
        json!("leaf")
    } else {
        let children: Vec<(String, Value)> = (0..width)
            .map(|i| (format!("key{}", i), generate_nested_json(depth - 1, width)))
            .collect();
        Value::Object(children.into_iter().collect())
    }
}

/// Generate a flat JSON object with given number of keys
fn generate_flat_json(num_keys: usize) -> Value {
    let entries: Vec<(String, Value)> = (0..num_keys)
        .map(|i| (format!("key{}", i), json!(i)))
        .collect();
    Value::Object(entries.into_iter().collect())
}

/// Generate a JSON array of given size
fn generate_array_json(size: usize) -> Value {
    let items: Vec<Value> = (0..size)
        .map(|i| json!({"id": i, "name": format!("item{}", i)}))
        .collect();
    Value::Array(items)
}

fn benchmark_flatten_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("flatten_json");

    // Small JSON (< 32 nodes)
    let small_json = json!({"a": 1, "b": {"c": 2, "d": 3}});
    let small_state = JsonTreeState::new();

    group.bench_function("small_vec", |b| {
        b.iter(|| flatten_json(black_box(&small_json), black_box(&small_state)))
    });

    group.bench_function("small_smallvec", |b| {
        b.iter(|| flatten_json_small(black_box(&small_json), black_box(&small_state)))
    });

    // Medium JSON (~100 nodes)
    let medium_json = generate_nested_json(3, 4);
    let medium_state = JsonTreeState::new();

    group.bench_function("medium_vec", |b| {
        b.iter(|| flatten_json(black_box(&medium_json), black_box(&medium_state)))
    });

    group.bench_function("medium_smallvec", |b| {
        b.iter(|| flatten_json_small(black_box(&medium_json), black_box(&medium_state)))
    });

    // Large JSON (~1000 nodes)
    let large_json = generate_nested_json(4, 5);
    let large_state = JsonTreeState::new();

    group.bench_function("large_vec", |b| {
        b.iter(|| flatten_json(black_box(&large_json), black_box(&large_state)))
    });

    group.finish();
}

fn benchmark_flatten_iterator_vs_vec(c: &mut Criterion) {
    let mut group = c.benchmark_group("flatten_iterator_vs_vec");

    let json = generate_nested_json(4, 4);
    let state = JsonTreeState::new();

    group.bench_function("vec_full", |b| {
        b.iter(|| {
            let nodes = flatten_json(black_box(&json), black_box(&state));
            black_box(nodes.len())
        })
    });

    group.bench_function("iter_full", |b| {
        b.iter(|| {
            let count = flatten_json_iter(black_box(&json), black_box(&state)).count();
            black_box(count)
        })
    });

    // Early termination - only take first 10
    group.bench_function("iter_take_10", |b| {
        b.iter(|| {
            let first_10: Vec<_> = flatten_json_iter(black_box(&json), black_box(&state))
                .take(10)
                .collect();
            black_box(first_10)
        })
    });

    group.finish();
}

fn benchmark_depth_limited(c: &mut Criterion) {
    let mut group = c.benchmark_group("depth_limited");

    let json = generate_nested_json(6, 3);
    let state = JsonTreeState::new();

    for depth in [1, 2, 3, 4] {
        group.bench_with_input(BenchmarkId::new("depth", depth), &depth, |b, &depth| {
            b.iter(|| flatten_json_to_depth(black_box(&json), black_box(&state), depth))
        });
    }

    group.finish();
}

fn benchmark_count_nodes(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_nodes");

    for size in [10, 100, 1000] {
        let json = generate_flat_json(size);
        group.bench_with_input(BenchmarkId::new("flat", size), &json, |b, json| {
            b.iter(|| count_nodes(black_box(json)))
        });
    }

    for depth in [2, 3, 4] {
        let json = generate_nested_json(depth, 4);
        let node_count = count_nodes(&json);
        group.bench_with_input(
            BenchmarkId::new("nested", format!("d{}_{}_nodes", depth, node_count)),
            &json,
            |b, json| b.iter(|| count_nodes(black_box(json))),
        );
    }

    group.finish();
}

fn benchmark_json_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_path");

    let json = json!({
        "users": [
            {"name": "Alice", "age": 30},
            {"name": "Bob", "age": 25}
        ]
    });

    // Path building
    group.bench_function("build_path", |b| {
        b.iter(|| black_box(JsonPath::root().key("users").index(0).key("name")))
    });

    // Path parsing
    group.bench_function("parse_path", |b| {
        b.iter(|| JsonPath::parse(black_box("users[0].name")))
    });

    // Path get value
    let path = JsonPath::root().key("users").index(0).key("name");
    group.bench_function("get_value", |b| b.iter(|| path.get(black_box(&json))));

    // Path conversions
    group.bench_function("to_json_pointer", |b| b.iter(|| path.to_json_pointer()));

    group.bench_function("to_jq_path", |b| b.iter(|| path.to_jq_path()));

    group.bench_function("to_bracket_notation", |b| {
        b.iter(|| path.to_bracket_notation())
    });

    group.finish();
}

fn benchmark_json_pointer(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_pointer");

    let json = generate_nested_json(5, 3);

    group.bench_function("get_by_pointer_shallow", |b| {
        b.iter(|| get_by_pointer(black_box(&json), "/key0"))
    });

    group.bench_function("get_by_pointer_deep", |b| {
        b.iter(|| get_by_pointer(black_box(&json), "/key0/key1/key2/key0"))
    });

    group.bench_function("path_to_pointer", |b| {
        b.iter(|| path_to_pointer(black_box("users[0].profile.name")))
    });

    group.bench_function("pointer_to_path", |b| {
        b.iter(|| pointer_to_path(black_box("/users/0/profile/name")))
    });

    group.finish();
}

fn benchmark_search_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_filter");

    let json = generate_array_json(100);
    let state = JsonTreeState::new();
    let nodes = flatten_json(&json, &state);

    group.bench_function("filter_nodes_match_many", |b| {
        b.iter(|| filter_nodes(black_box(&nodes), "item"))
    });

    group.bench_function("filter_nodes_match_few", |b| {
        b.iter(|| filter_nodes(black_box(&nodes), "item50"))
    });

    group.bench_function("filter_nodes_no_match", |b| {
        b.iter(|| filter_nodes(black_box(&nodes), "nonexistent"))
    });

    group.bench_function("find_path", |b| {
        b.iter(|| find_path(black_box(&json), |v| v == &json!("item50")))
    });

    group.bench_function("find_all_paths", |b| {
        b.iter(|| find_all_paths(black_box(&json), |v| matches!(v, Value::Number(_))))
    });

    group.finish();
}

fn benchmark_parse_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_json");

    let small_str = r#"{"name": "test"}"#;
    let medium_str = serde_json::to_string(&generate_flat_json(50)).unwrap();
    let large_str = serde_json::to_string(&generate_nested_json(3, 5)).unwrap();

    group.bench_function("small", |b| b.iter(|| parse_json(black_box(small_str))));

    group.bench_function("medium", |b| b.iter(|| parse_json(black_box(&medium_str))));

    group.bench_function("large", |b| b.iter(|| parse_json(black_box(&large_str))));

    group.bench_function("invalid_early_fail", |b| {
        b.iter(|| parse_json(black_box("not json")))
    });

    group.finish();
}

fn benchmark_state_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_operations");

    let json = generate_nested_json(4, 4);
    let nodes = flatten_json(&json, &JsonTreeState::new());
    let paths: Vec<_> = nodes.iter().map(|n| n.path.clone()).collect();

    group.bench_function("toggle_100", |b| {
        b.iter(|| {
            let mut state = JsonTreeState::new();
            for path in paths.iter().take(100) {
                state.toggle(path);
            }
            black_box(state)
        })
    });

    group.bench_function("is_collapsed_100", |b| {
        let mut state = JsonTreeState::new();
        for path in paths.iter().take(50) {
            state.collapse(path);
        }
        b.iter(|| {
            let mut count = 0;
            for path in paths.iter().take(100) {
                if state.is_collapsed(path) {
                    count += 1;
                }
            }
            black_box(count)
        })
    });

    group.bench_function("collapse_deep", |b| {
        b.iter(|| {
            let mut state = JsonTreeState::new();
            state.collapse_deep(black_box(&json), 2);
            black_box(state)
        })
    });

    group.bench_function("builder_collapsed_at_depth", |b| {
        b.iter(|| {
            JsonTreeState::builder()
                .collapsed_at_depth(black_box(&json), 2)
                .build()
        })
    });

    group.finish();
}

fn benchmark_diff(c: &mut Criterion) {
    use lib_json_tree::{count_diff_changes, diff_json, filter_diff_changes};

    let mut group = c.benchmark_group("diff");

    // Small diff - few changes
    let small_old = json!({"a": 1, "b": 2, "c": 3});
    let small_new = json!({"a": 1, "b": 3, "c": 3});

    group.bench_function("small_single_change", |b| {
        b.iter(|| diff_json(black_box(&small_old), black_box(&small_new)))
    });

    // Medium diff - nested objects
    let medium_old = json!({
        "user": {"name": "Alice", "age": 30, "email": "alice@example.com"},
        "settings": {"theme": "dark", "notifications": true}
    });
    let medium_new = json!({
        "user": {"name": "Alice", "age": 31, "email": "alice@new.com"},
        "settings": {"theme": "light", "notifications": true, "language": "en"}
    });

    group.bench_function("medium_nested", |b| {
        b.iter(|| diff_json(black_box(&medium_old), black_box(&medium_new)))
    });

    // Large diff - arrays
    let large_old = generate_array_json(50);
    let large_new = generate_array_json(60);

    group.bench_function("large_array", |b| {
        b.iter(|| diff_json(black_box(&large_old), black_box(&large_new)))
    });

    // Count and filter diff changes
    let diffs = diff_json(&medium_old, &medium_new);
    group.bench_function("count_changes", |b| {
        b.iter(|| count_diff_changes(black_box(&diffs)))
    });

    group.bench_function("filter_changes", |b| {
        b.iter(|| filter_diff_changes(black_box(&diffs)))
    });

    group.finish();
}

fn benchmark_highlight(c: &mut Criterion) {
    use lib_json_tree::{format_node_display, get_highlight_spans, JsonTreeNode, JsonValueType};
    use std::borrow::Cow;

    let mut group = c.benchmark_group("highlight");

    let string_node = JsonTreeNode {
        path: Cow::Owned("name".to_string()),
        depth: 1,
        key: Some(Cow::Owned("name".to_string())),
        value_type: JsonValueType::String,
        value_str: Some(Cow::Owned("\"Alice\"".to_string())),
        child_count: 0,
        collapsible: false,
        is_collapsed: false,
    };

    let text = "\"name\": \"Alice\"";

    group.bench_function("get_highlight_spans", |b| {
        b.iter(|| get_highlight_spans(black_box(&string_node), black_box(text)))
    });

    group.bench_function("format_node_display", |b| {
        b.iter(|| format_node_display(black_box(&string_node)))
    });

    // Object node
    let object_node = JsonTreeNode {
        path: Cow::Owned("data".to_string()),
        depth: 1,
        key: Some(Cow::Owned("data".to_string())),
        value_type: JsonValueType::Object,
        value_str: None,
        child_count: 5,
        collapsible: true,
        is_collapsed: true,
    };

    group.bench_function("format_collapsed_object", |b| {
        b.iter(|| format_node_display(black_box(&object_node)))
    });

    group.finish();
}

fn benchmark_virtual_scroll(c: &mut Criterion) {
    use lib_json_tree::{
        calculate_visible_range, find_node_index, get_visible_nodes, scroll_to_node,
    };

    let mut group = c.benchmark_group("virtual_scroll");

    // Visible range calculations
    group.bench_function("calculate_visible_range", |b| {
        b.iter(|| {
            calculate_visible_range(
                black_box(10000),
                black_box(500),
                black_box(50),
                black_box(10),
            )
        })
    });

    // Get visible nodes from large array
    let large_vec: Vec<i32> = (0..10000).collect();
    group.bench_function("get_visible_nodes_10k", |b| {
        b.iter(|| get_visible_nodes(black_box(&large_vec), 500, 50, 10))
    });

    // Scroll to node calculations
    group.bench_function("scroll_to_node", |b| {
        b.iter(|| scroll_to_node(black_box(5000), black_box(50), black_box(100)))
    });

    // Find node index in flattened tree
    let json = generate_nested_json(4, 5);
    let state = JsonTreeState::new();
    let nodes = flatten_json(&json, &state);

    group.bench_function("find_node_index", |b| {
        b.iter(|| find_node_index(black_box(&nodes), black_box("key2.key1.key0")))
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_flatten_json,
    benchmark_flatten_iterator_vs_vec,
    benchmark_depth_limited,
    benchmark_count_nodes,
    benchmark_json_path,
    benchmark_json_pointer,
    benchmark_search_filter,
    benchmark_parse_json,
    benchmark_state_operations,
    benchmark_diff,
    benchmark_highlight,
    benchmark_virtual_scroll,
);

criterion_main!(benches);
