# Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
# Automated VHS Recording: peri CLI Quickstart
# Output target: assets/demos/peri-quickstart.gif

Output assets/demos/peri-quickstart.gif

# Terminal and visual aesthetics (Campbell Theme: deep #0C0C0C dark palette)
Set Shell "bash"
Set FontSize 15
Set Width 1200
Set Height 620
Set Padding 20
Set Framerate 30
Set PlaybackSpeed 1.0
Set Theme { "name": "Campbell", "background": "#0C0C0C", "foreground": "#CCCCCC", "cursor": "#FFFFFF", "black": "#0C0C0C", "blue": "#0037DA", "cyan": "#3A96DD", "green": "#13A10E", "magenta": "#881798", "red": "#C50F1F", "white": "#CCCCCC", "yellow": "#C19C00", "brightBlack": "#767676", "brightBlue": "#3B78FF", "brightCyan": "#61D6D6", "brightGreen": "#16C60C", "brightMagenta": "#B4009E", "brightRed": "#E74856", "brightWhite": "#F2F2F2", "brightYellow": "#F9F1A5" }
Set WindowBar "Colorful"
Set BorderRadius 8

# Sandbox initialization (hidden from recording)
Hide
Type "export PS1='perihelion:~$ ' && cd $HOME && rm -rf ~/.cache/perihelion-vhs-demo && mkdir -p ~/.cache/perihelion-vhs-demo && cd ~/.cache/perihelion-vhs-demo && clear"
Enter
Sleep 1s
Show

# 1. Print CLI help banner
Type "peri --help"
Sleep 500ms
Enter
Sleep 3s

# 2. Initialize a Cyberpunk 2077 workspace profile
Type "peri init cyberpunk_2077 --name 'night-city-overhaul'"
Sleep 500ms
Enter
Sleep 3s

# 3. Inspect generated declarative manifest
Type "cat peri.toml"
Sleep 500ms
Enter
Sleep 2.5s

# 4. Validate dependency DAG and Virtual File Index (VFI)
Type "peri check"
Sleep 500ms
Enter
Sleep 4s

# Sandbox cleanup (hidden from recording)
Hide
Type "rm -rf ~/.cache/perihelion-vhs-demo"
Enter
Sleep 500ms

