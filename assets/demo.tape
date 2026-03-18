Output assets/demo.gif

Set Shell "zsh"
Set Width 1400
Set Height 800
Set FontSize 14
Set Theme "Catppuccin Mocha"
Set Padding 20
Set TypingSpeed 40ms

Hide
Type "cd /tmp/giff-demo"
Enter
Sleep 500ms
Show

# Launch giff
Type "giff HEAD~1 HEAD"
Enter
Sleep 3s

# Browse through files
Sleep 1s
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms

# Scroll down in the diff pane
Type "l"
Sleep 500ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 300ms
Type "j"
Sleep 1s

# Go back to file list and navigate to another file
Type "h"
Sleep 500ms
Type "j"
Sleep 800ms

# Scroll diff
Type "l"
Sleep 500ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 1s

# Toggle unified view
Type "u"
Sleep 2s

# Scroll in unified view
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 200ms
Type "j"
Sleep 1s

# Toggle back to side-by-side
Type "u"
Sleep 1.5s

# Switch to another file
Type "h"
Sleep 500ms
Type "j"
Sleep 800ms
Type "j"
Sleep 800ms

# Toggle light theme
Type "t"
Sleep 2s

# Toggle back to dark
Type "t"
Sleep 1.5s

# Show help overlay
Type "?"
Sleep 3s

# Close help and quit
Type "?"
Sleep 1s
Type "q"
Sleep 1s
