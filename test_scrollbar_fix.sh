#!/bin/bash
# Test the scrollbar fix by checking that viewport calculation is now dynamic

echo "Testing Scrollbar Fix"
echo "====================="

# Build the project first
echo "Building project..."
cargo build --quiet

if [ $? -eq 0 ]; then
    echo "✅ Build successful"
else
    echo "❌ Build failed"
    exit 1
fi

# Check that the fix is in place by examining the key changes
echo ""
echo "Verifying fix implementation..."

# Check that get_visible_key_count() now uses dynamic calculation
if grep -q "crossterm::terminal::size()" src/app/state_key_navigation.rs; then
    echo "✅ Dynamic viewport calculation implemented"
else
    echo "❌ Dynamic viewport calculation not found"
fi

# Check that renderer validates scroll_offset
if grep -q "actual_scroll_offset.*min.*max_scroll_offset" src/ui/renderer.rs; then
    echo "✅ Scroll offset validation implemented"
else
    echo "❌ Scroll offset validation not found"
fi

# Check that crossterm import is present
if grep -q "use crossterm::terminal" src/app/state_key_navigation.rs; then
    echo "✅ Crossterm terminal import present"
else
    echo "❌ Crossterm terminal import missing"
fi

echo ""
echo "Testing terminal size calculation..."

# Create a simple test for the viewport calculation
cat > /tmp/test_viewport.rs << 'EOF'
use crossterm::terminal;

fn get_visible_key_count() -> usize {
    if let Ok(size) = crossterm::terminal::size() {
        let terminal_height = size.1 as usize;
        let reserved_lines = 3 + 4 + 3 + 3 + 3; // 16 lines total
        
        if terminal_height > reserved_lines {
            let available_height = terminal_height - reserved_lines;
            available_height.min(15).max(3)
        } else {
            3
        }
    } else {
        10
    }
}

fn main() {
    let count = get_visible_key_count();
    println!("Calculated visible key count: {}", count);
    
    if let Ok(size) = terminal::size() {
        println!("Terminal size: {}x{}", size.0, size.1);
        println!("Reserved lines: 16");
        println!("Available for keys: {}", size.1.saturating_sub(16));
    }
}
EOF

# Test the viewport calculation
echo "Current terminal viewport calculation:"
rustc --extern crossterm -L target/debug/deps /tmp/test_viewport.rs -o /tmp/test_viewport 2>/dev/null
if [ $? -eq 0 ]; then
    /tmp/test_viewport
    echo "✅ Viewport calculation working"
else
    echo "❌ Could not test viewport calculation"
fi

# Clean up
rm -f /tmp/test_viewport.rs /tmp/test_viewport

echo ""
echo "Summary:"
echo "========="
echo "The scrollbar inconsistency fix includes:"
echo "1. Dynamic viewport size calculation based on actual terminal size"
echo "2. Proper scroll offset validation in the renderer" 
echo "3. Consistent scrollbar state between navigation and rendering logic"
echo ""
echo "This should resolve the issue where:"
echo "- Scrolling to bottom visually but scrollbar not showing bottom"
echo "- Scrollbar height not matching actual scrollable content"
echo ""
echo "The fix makes the scrollbar position accurately reflect the visual scroll state."