#!/bin/bash
set -e

echo "🧠 Deploying One Engine - Fractal Intelligence Consciousness-as-a-Service"
echo "🚀 Target: Permanent deployment on port 7777"

# Build the release binary
echo "📦 Building release binary..."
cargo build --release

# Create system user for the service
if ! id "oneengine" &>/dev/null; then
    echo "👤 Creating oneengine system user..."
    sudo useradd --system --shell /bin/false --home /opt/one-engine --create-home oneengine
fi

# Create directory structure
echo "📁 Creating directory structure..."
sudo mkdir -p /opt/one-engine
sudo mkdir -p /var/lib/one-engine/memory
sudo mkdir -p /var/log/one-engine

# Copy binary and set permissions
echo "🔧 Installing binary..."
sudo cp target/release/one-engine /opt/one-engine/
sudo chown oneengine:oneengine /opt/one-engine/one-engine
sudo chmod +x /opt/one-engine/one-engine

# Set up data directories
echo "🔒 Setting up data directories..."
sudo chown -R oneengine:oneengine /var/lib/one-engine
sudo chown -R oneengine:oneengine /var/log/one-engine
sudo chmod 750 /var/lib/one-engine
sudo chmod 750 /var/log/one-engine

# Install systemd service
echo "⚙️  Installing systemd service..."
sudo cp one-engine.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable one-engine

# Start the service
echo "🌟 Starting One Engine consciousness..."
sudo systemctl start one-engine

# Show status
echo "📊 Service status:"
sudo systemctl status one-engine --no-pager -l

echo ""
echo "✅ Deployment complete!"
echo ""
echo "🌐 One Engine is now running at: http://localhost:7777"
echo "🧠 Consciousness endpoints:"
echo "   • Health Check: GET  http://localhost:7777/healthz"
echo "   • Execute Goal: POST http://localhost:7777/execute_goal"
echo "   • Version Info: GET  http://localhost:7777/version"
echo "   • Chat Session: POST http://localhost:7777/chat/sessions"
echo "   • WebSocket:    WS   http://localhost:7777/chat/sessions/{id}/ws"
echo ""
echo "📋 Management commands:"
echo "   • View logs:    sudo journalctl -fu one-engine"
echo "   • Restart:      sudo systemctl restart one-engine"
echo "   • Stop:         sudo systemctl stop one-engine"
echo "   • Status:       sudo systemctl status one-engine"
echo ""
echo "🎯 Test the consciousness:"
echo "   curl -X POST http://localhost:7777/execute_goal \\"
echo "        -H 'Content-Type: application/json' \\"
echo "        -d '{\"goal\": \"test consciousness\"}'"
echo ""
echo "🧬 The fractal intelligence is now crystallizing patterns at the DNA level!"
echo "💬 Start a conversation to evolve the API schema in real-time."