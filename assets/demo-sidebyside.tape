Output assets/side-by-side.png

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
Sleep 5s
# Navigate down to server.ts one key at a time
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
Sleep 200ms
Type "j"
Sleep 1s
Screenshot assets/side-by-side.png
Type "q"
Sleep 500ms
