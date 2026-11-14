Output "/tmp/ducky-screenshots.gif"

Set Shell "bash"
Set FontSize 16
Set Width 1500
Set Height 860
Set Padding 24
Set Theme "TokyoNight"
Set TypingSpeed 40ms

Hide
Type "unset NO_COLOR; export TERM=xterm-256color; clear; target/debug/ducky --config dev/ducky.dev.toml --no-effects" Enter
Sleep 2s
Show
Sleep 500ms
Screenshot assets/profiles.png
Sleep 1s

Hide
Enter
Sleep 2s
Type "jjjj"
Enter
Sleep 2s
Type "ll"
Type "c"
Sleep 300ms
Type "j"
Enter
Type "j"
Type "c"
Sleep 300ms
Type "jj"
Enter
Type "l"
Type "c"
Sleep 300ms
Ctrl+U
Type "`total * 1.2`"
Enter
Type "jj"
Type "h"
Type "c"
Sleep 300ms
Type "j"
Enter
Type "j"
Sleep 500ms
Show
Sleep 500ms
Screenshot assets/draft-edits.png
Sleep 1s

Hide
Type "c"
Sleep 500ms
Show
Sleep 500ms
Screenshot assets/enum-dropdown.png
Sleep 1s

Hide
Escape
Sleep 300ms
Type "t"
Sleep 1s
Show
Sleep 500ms
Screenshot assets/structure.png
Sleep 1s

Hide
Type "t"
Ctrl+N
Sleep 500ms
Type "i"
Type "SELECT c.email, o.status, count(*) AS orders, sum(o.total) AS total"
Enter
Type "FROM customers c JOIN orders o ON o.customer_id = c.id"
Enter
Type "GROUP BY 1, 2 ORDER BY total DESC LIMIT 20;"
Escape
Sleep 300ms
Ctrl+E
Sleep 1500ms
Show
Sleep 500ms
Screenshot assets/query.png
Sleep 1s

Sleep 500ms
