//! Basic sync example demonstrating the protocol
//!
//! Run with: cargo run --example basic_sync

use lib_tarminal_sync::*;
use uuid::Uuid;

fn main() {
    println!("🔄 Tarminal Sync Protocol - Basic Example\n");

    // Create two devices
    let device1 = Uuid::new_v4();
    let device2 = Uuid::new_v4();

    println!("📱 Device 1: {}", device1);
    println!("📱 Device 2: {}\n", device2);

    // Device 1 creates a workspace
    let workspace = SyncableWorkspace {
        id: Uuid::new_v4(),
        name: "My Project".to_string(),
        icon: Some("🚀".to_string()),
        session_ids: vec![],
        active_session_id: None,
        sync_metadata: SyncMetadata::new(device1),
    };

    println!("✨ Device 1 created workspace: '{}'", workspace.name);
    println!("   Version: {:?}", workspace.sync_metadata.version);

    // Serialize to JSON (ready to send over any transport)
    let msg = SyncMessage::WorkspaceUpdate {
        workspace: workspace.clone(),
    };
    let json = serde_json::to_string_pretty(&msg).unwrap();
    println!("\n📤 Sending workspace update:\n{}\n", json);

    // Device 2 receives the update
    let received: SyncMessage = serde_json::from_str(&json).unwrap();
    println!("📥 Device 2 received workspace update");

    if let SyncMessage::WorkspaceUpdate { workspace: ws } = received {
        println!("   Workspace: '{}'", ws.name);
        println!("   Created by: {}", ws.sync_metadata.origin_device_id);
    }

    // Device 2 modifies the workspace
    let mut workspace_v2 = workspace.clone();
    workspace_v2.name = "My Awesome Project".to_string();
    workspace_v2.sync_metadata.touch(device2);

    println!("\n✏️  Device 2 modified workspace: '{}'", workspace_v2.name);
    println!("   Version: {:?}", workspace_v2.sync_metadata.version);

    // Check for conflicts
    if workspace
        .sync_metadata
        .concurrent_with(&workspace_v2.sync_metadata)
    {
        println!("\n⚠️  Concurrent modification detected!");
        println!("   Merging using Last-Writer-Wins...");

        let merged_metadata = workspace.sync_metadata.merged(&workspace_v2.sync_metadata);
        println!("   Merged version: {:?}", merged_metadata.version);
    }

    // Grid delta example
    println!("\n🖥️  Terminal Grid Delta Example");

    let delta = GridDelta {
        operations: vec![
            GridOperation::CursorMove { x: 10, y: 5 },
            GridOperation::SetCells {
                row: 5,
                start_col: 10,
                cells: vec![
                    Cell {
                        char: 'H',
                        fg: TerminalColor::Named {
                            color: NamedColor::BrightGreen,
                        },
                        bg: TerminalColor::Default,
                        bold: true,
                        ..Default::default()
                    },
                    Cell {
                        char: 'i',
                        fg: TerminalColor::Named {
                            color: NamedColor::BrightGreen,
                        },
                        bg: TerminalColor::Default,
                        bold: true,
                        ..Default::default()
                    },
                ],
            },
        ],
        base_version: 1,
        new_version: 2,
    };

    let delta_json = serde_json::to_string_pretty(&delta).unwrap();
    println!("   Delta operations:\n{}", delta_json);

    // Signaling example
    println!("\n📡 Signaling Server Messages");

    let pairing_msg = SignalingMessage::CreatePairingCode;
    let pairing_json = serde_json::to_string(&pairing_msg).unwrap();
    println!("   → {}", pairing_json);

    let code_response = SignalingMessage::PairingCode {
        code: "ABC123".to_string(),
    };
    let code_json = serde_json::to_string(&code_response).unwrap();
    println!("   ← {}", code_json);

    println!("\n✅ Example completed!");
}
