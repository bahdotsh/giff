Output assets/unified-view.png

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
# Navigate to database.ts (4th file)
Type "jjj"
Sleep 500ms
# Toggle unified view
Type "u"
Sleep 1s
Screenshot assets/unified-view.png
Type "q"
Sleep 500ms
