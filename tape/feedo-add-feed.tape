# Feedo Add Feed Screenshot
Output website/public/screenshots/feedo-add-feed.png

Set Width 1200
Set Height 700
Set FontSize 16
Set FontFamily "JetBrains Mono"
Set Theme "Dracula"
Set Padding 20
Set WindowBar Colorful
Set WindowBarSize 40
Set TypingSpeed 50ms

Hide
Type "cd ~/dev/feedo && clear"
Enter
Sleep 500ms
Show

Type "./target/release/feedo"
Enter
Sleep 3s

Type "n"
Sleep 1s
Type "https://blog.rust-lang.org/feed.xml"
Sleep 1s

Screenshot website/public/screenshots/feedo-add-feed.png
Escape
Sleep 300ms
Type "q"
