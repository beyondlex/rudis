# Tree View Fix for Overlapping Key Paths

## Problem Description

When Redis contains keys with overlapping paths, such as:
- `user:1` (hash type)
- `user:1:name` (string type)

Only one of these keys would be displayed in the tree view. Specifically, the `user:1:name` key would not appear because the tree structure couldn't handle a node that is both a Redis key AND has children.

## Root Cause

The original tree implementation had a fundamental limitation:
- A tree node was either a "leaf" (actual Redis key) OR an "intermediate" node (folder)
- When inserting `user:1`, it created a leaf node at path `["user", "1"]`
- When later inserting `user:1:name`, the algorithm failed because it tried to traverse through the "1" node to add "name" as a child
- But "1" was already marked as a leaf node, so it couldn't have children

## Solution

Modified the tree structure to support **hybrid nodes** - nodes that can be both Redis keys AND have children:

### Changes Made

1. **Updated TreeNode structure** (`src/ui/tree_view.rs`):
   - Added `is_key()` method to check if a node represents a Redis key
   - Modified `is_leaf()` to mean "key with no children" vs "key that can have children"
   - Updated `insert_key()` to handle hybrid nodes properly

2. **Updated tree insertion logic**:
   ```rust
   if is_last {
       // This is the actual key
       if let Some(existing_node) = current.children.get_mut(segment) {
           // Node already exists, just add key info to it (hybrid node)
           existing_node.key_info = Some(key_info.clone());
           existing_node.full_path = Some(key_info.name.clone());
       } else {
           // Create new leaf node
           let leaf = TreeNode::new_leaf(segment.clone(), key_info.clone(), i + 1);
           current.children.insert(segment.clone(), leaf);
       }
   }
   ```

3. **Updated display logic** (`src/ui/renderer.rs`):
   - Changed condition from `is_leaf` to `is_key` to display all Redis keys
   - Added folder indicators for keys that also have children
   - Hybrid nodes now show: `📋 1 📁` (key icon + name + folder indicator)

4. **Updated navigation logic** (`src/app/state.rs`):
   - Changed navigation to use `is_key` instead of `is_leaf`
   - Allows selection of both pure keys and hybrid keys

### Display Examples

**Before (broken):**
```
📂 user/
  📋 1           <- Only shows user:1 (hash)
                 <- user:1:name is missing
```

**After (fixed):**
```
📂 user/
  📋 1 📁        <- Shows user:1 (hash) with folder indicator
    🔤 name      <- Shows user:1:name (string)
```

## Key Benefits

1. **No data loss**: All Redis keys are now visible in tree view
2. **Clear visual indication**: Hybrid nodes show both key icon and folder indicator
3. **Proper navigation**: Can select and view both types of keys
4. **Backward compatible**: Pure keys and folders work as before

## Files Modified

- `src/ui/tree_view.rs` - Core tree structure and insertion logic
- `src/ui/renderer.rs` - Display logic for tree view
- `src/app/state.rs` - Navigation logic updates

## Testing

To test the fix:
1. Create Redis keys with overlapping paths:
   ```bash
   redis-cli HSET user:1 field1 value1
   redis-cli SET user:1:name "John Doe"
   ```
2. Run rudis and switch to tree view (`t` key)
3. Both keys should now be visible with proper hierarchy