//! Simple client demonstrating connection to signaling server
//!
//! Run signaling server first:
//!   cd crates/tarminal-signaling-server && cargo run
//!
//! Then run this client:
//!   cargo run --example simple_client

use lib_tarminal_sync::*;
use std::io::{self, Write};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Tarminal Sync - Simple Client Demo\n");

    let device_id = Uuid::new_v4();
    println!("📱 Device ID: {}\n", device_id);

    // In real implementation, connect to WebSocket server
    println!("To connect to signaling server:");
    println!("  1. Start server: cd crates/tarminal-signaling-server && cargo run");
    println!("  2. Server runs on: ws://localhost:8080/ws\n");

    println!("What would you like to do?");
    println!("  1. Create pairing code");
    println!("  2. Use pairing code");
    println!("  3. Send workspace update");
    println!("  4. See example messages\n");

    print!("Choice (1-4): ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;

    match choice.trim() {
        "1" => {
            println!("\n📤 Creating pairing code...");
            let msg = SignalingMessage::CreatePairingCode;
            let json = serde_json::to_string_pretty(&msg)?;
            println!("Send this to server:\n{}", json);
            println!("\n📥 Server would respond with:");
            let response = SignalingMessage::PairingCode {
                code: "X7K9M2".to_string(),
            };
            println!("{}", serde_json::to_string_pretty(&response)?);
        }

        "2" => {
            print!("\nEnter pairing code: ");
            io::stdout().flush()?;
            let mut code = String::new();
            io::stdin().read_line(&mut code)?;

            println!("\n📤 Using pairing code...");
            let msg = SignalingMessage::UsePairingCode {
                code: code.trim().to_uppercase(),
            };
            let json = serde_json::to_string_pretty(&msg)?;
            println!("Send this to server:\n{}", json);
        }

        "3" => {
            println!("\n📤 Sending workspace update...");

            let workspace = SyncableWorkspace {
                id: Uuid::new_v4(),
                name: "Demo Workspace".to_string(),
                icon: Some("🚀".to_string()),
                session_ids: vec![],
                active_session_id: None,
                sync_metadata: SyncMetadata::new(device_id),
            };

            let msg = SyncMessage::WorkspaceUpdate { workspace };
            let json = serde_json::to_string_pretty(&msg)?;
            println!("Sync message:\n{}", json);
        }

        "4" => {
            println!("\n📋 Example Messages:\n");

            println!("1️⃣ Register with server:");
            let msg = SignalingMessage::Register {
                device_id: device_id.to_string(),
            };
            println!("{}\n", serde_json::to_string_pretty(&msg)?);

            println!("2️⃣ Create pairing code:");
            let msg = SignalingMessage::CreatePairingCode;
            println!("{}\n", serde_json::to_string_pretty(&msg)?);

            println!("3️⃣ Workspace update:");
            let workspace = SyncableWorkspace {
                id: Uuid::new_v4(),
                name: "Example".to_string(),
                icon: None,
                session_ids: vec![],
                active_session_id: None,
                sync_metadata: SyncMetadata::new(device_id),
            };
            let msg = SyncMessage::WorkspaceUpdate { workspace };
            println!("{}\n", serde_json::to_string_pretty(&msg)?);

            println!("4️⃣ Grid delta:");
            let delta = GridDelta {
                operations: vec![GridOperation::CursorMove { x: 10, y: 5 }],
                base_version: 1,
                new_version: 2,
            };
            println!("{}\n", serde_json::to_string_pretty(&delta)?);
        }

        _ => println!("Invalid choice"),
    }

    println!("\n✅ Demo completed!");
    println!("\nFor full WebSocket implementation, see:");
    println!("  - apps/tarminal-native-macos/Packages/TarminalWebSync/");
    println!("  - examples/web_client.html");

    Ok(())
}
