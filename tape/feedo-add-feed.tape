# Feedo Add Feed Screenshot
Output website/public/screenshots/feedo-add-feed.png

Set Width 1200
Set Height 700
Set FontSize 14
Set FontFamily "monospace"
Set Theme "Dracula"
Set Padding 10
Set WindowBar Colorful
Set WindowBarSize 40
Set TypingSpeed 50ms
Set LetterSpacing 0

Hide
Type "clear"
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
