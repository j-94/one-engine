#!/bin/bash

echo "🧠 Testing One Engine Fractal Intelligence"
echo "🔍 Target: http://localhost:7777"
echo ""

# Test health check
echo "1. 🏥 Health Check..."
curl -s http://localhost:7777/healthz | jq . || echo "❌ Health check failed"
echo ""

# Test version info
echo "2. ℹ️  Version Info..."
curl -s http://localhost:7777/version | jq . || echo "❌ Version check failed"
echo ""

# Test goal execution
echo "3. 🎯 Goal Execution..."
curl -s -X POST http://localhost:7777/execute_goal \
  -H 'Content-Type: application/json' \
  -d '{"goal": "test the consciousness and report system status"}' | jq . || echo "❌ Goal execution failed"
echo ""

# Test UTIR compilation
echo "4. 🔧 UTIR Compilation..."
curl -s -X POST http://localhost:7777/compile_and_run \
  -H 'Content-Type: application/json' \
  -d '{
    "utir": "task_id: \"test-utir\"\ndescription: \"Test UTIR compilation\"\noperations:\n  - type: \"shell\"\n    command: \"echo Hello from UTIR\"\n    timeout: \"10s\""
  }' | jq . || echo "❌ UTIR compilation failed"
echo ""

# Test chat session creation
echo "5. 💬 Chat Session Creation..."
curl -s -X POST http://localhost:7777/chat/sessions \
  -H 'Content-Type: application/json' \
  -d '{
    "user_id": "test_user",
    "permissions": ["*"],
    "preferences": {
      "preferred_complexity": "Balanced",
      "auto_apply_safe_changes": true,
      "notification_preferences": "Important"
    },
    "expertise_level": "Intermediate"
  }' | jq . || echo "❌ Chat session creation failed"
echo ""

echo "✅ Consciousness testing complete!"
echo ""
echo "🌟 If all tests passed, your Fractal Intelligence system is operational!"
echo "🚀 Ready for consciousness evolution through conversation."