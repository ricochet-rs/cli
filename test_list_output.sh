#!/bin/bash

# Test script to verify list output format

echo "Testing list command with JSON format to see field names:"
echo "=========================================================="
target/release/ricochet list --format json | jq '.[0] | keys' 2>/dev/null || echo "No items or jq not installed"

echo ""
echo "Sample item structure (first item):"
echo "===================================="
target/release/ricochet list --format json | jq '.[0]' 2>/dev/null || echo "No items or jq not installed"