#!/usr/bin/env bash

# Test script to verify the tree view fix
# This script sets up Redis with the problematic keys and tests the display

echo "Testing tree view fix for overlapping keys..."

# Note: This is a manual test that would require a running Redis instance
# The actual fix is in the code - here's what it should handle:

echo "Test case 1: Keys that have overlapping paths"
echo "- Key 'user:1' (hash type)"
echo "- Key 'user:1:name' (string type)"
echo ""
echo "Expected behavior:"
echo "📂 user/"
echo "  📋 1 📁        <- This shows user:1 (hash) with folder indicator (has children)"
echo "    🔤 name      <- This shows user:1:name (string)"
echo ""
echo "The fix allows 'user:1' to be both a Redis key AND have children,"
echo "so both keys should be visible in the tree view."

echo ""
echo "To test manually:"
echo "1. Start Redis: redis-server"
echo "2. Add test data:"
echo "   redis-cli HSET user:1 field1 value1"
echo "   redis-cli SET user:1:name \"John Doe\""
echo "3. Run rudis and check tree view displays both keys"