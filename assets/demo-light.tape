Output assets/light-theme.png

Set Shell "zsh"
Set Width 1400
Set Height 800
Set FontSize 14
Set Theme "Catppuccin Latte"
Set Padding 20

Hide
Type "cd /tmp/giff-demo-bugfix"
Enter
Sleep 500ms
Show

Type "/Users/goku/projects/giff/target/release/giff HEAD~1 HEAD --theme light"
Enter
Sleep 4s
Screenshot assets/light-theme.png
Type "q"
Sleep 500ms
