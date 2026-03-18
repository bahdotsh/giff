Output assets/syntax-highlight.png

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
# Navigate to src/auth.rs (3rd file)
Type "jj"
Sleep 1s
Screenshot assets/syntax-highlight.png
Type "q"
Sleep 500ms
