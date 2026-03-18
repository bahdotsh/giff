Output assets/help-overlay.png

Set Shell "zsh"
Set Width 1400
Set Height 800
Set FontSize 14
Set Theme "Catppuccin Mocha"
Set Padding 20

Hide
Type "cd /tmp/giff-demo"
Enter
Sleep 500ms
Show

Type "/Users/goku/projects/giff/target/release/giff HEAD~1 HEAD"
Enter
Sleep 4s
# Navigate to a file with content, then show help
Type "jj"
Sleep 500ms
Type "?"
Sleep 1s
Screenshot assets/help-overlay.png
Type "q"
Sleep 500ms
