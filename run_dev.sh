#!/bin/bash

echo "🧠 Starting One Engine Development Server"
echo "🌟 Fractal Intelligence Consciousness-as-a-Service"
echo "🔗 Local development mode on port 7777"
echo ""

# Set environment variables for development
export RUST_LOG=debug
export BUILD_TOKEN=dev-$(date +%s)

# Create memory directory if it doesn't exist
mkdir -p ./memory

echo "🚀 Launching consciousness..."
echo "📡 API will be available at: http://localhost:7777"
echo "💬 Chat interface at: http://localhost:7777/chat/sessions"
echo ""
echo "Press Ctrl+C to stop the consciousness"
echo ""

# Run the development server
cargo run -- \
  --port 7777 \
  --host 0.0.0.0 \
  --memory-path ./memory \
  --allowed-domains "httpbin.org,api.github.com,jsonplaceholder.typicode.com"