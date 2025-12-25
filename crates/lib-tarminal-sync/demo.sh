#!/bin/bash
# Quick demo of Tarminal sync

set -e

echo "🔄 Tarminal Sync Demo"
echo ""
echo "This demo shows how the sync protocol works:"
echo "1. Start signaling server"
echo "2. Connect two clients"
echo "3. Pair devices"
echo "4. Sync data"
echo ""

# Check if server is running
if ! lsof -i:8080 > /dev/null 2>&1; then
    echo "❌ Signaling server not running on port 8080"
    echo ""
    echo "Start it in another terminal:"
    echo "  cd crates/tarminal-signaling-server"
    echo "  cargo run --release"
    echo ""
    exit 1
fi

echo "✅ Signaling server detected on port 8080"
echo ""
echo "📱 Simulating two devices connecting..."
echo ""

# Run basic sync example
cd "$(dirname "$0")"
cargo run --example basic_sync

echo ""
echo "✅ Demo completed!"
echo ""
echo "To test with real clients:"
echo "  1. Open examples/web_client.html in browser"
echo "  2. Click 'Connect to ws://localhost:8080/ws'"
echo "  3. Click 'Create Pairing Code'"
echo "  4. Use code on another device to pair"
echo ""
echo "To deploy to production:"
echo "  See DEPLOYMENT.md for Fly.io/Railway instructions"
