# Perihelion - Game-agnostic native Linux mod manager and VFS orchestrator.
# Automated VHS Recording: helion Interactive TUI Dashboard
# Output target: assets/demos/helion-dashboard.gif

Output assets/demos/helion-dashboard.gif

# Terminal and visual aesthetics (Campbell Theme: deep #0C0C0C dark palette)
Set Shell "bash"
Set FontSize 15
Set Width 1200
Set Height 640
Set Padding 20
Set Framerate 30
Set PlaybackSpeed 1.0
Set Theme { "name": "Campbell", "background": "#0C0C0C", "foreground": "#CCCCCC", "cursor": "#FFFFFF", "black": "#0C0C0C", "blue": "#0037DA", "cyan": "#3A96DD", "green": "#13A10E", "magenta": "#881798", "red": "#C50F1F", "white": "#CCCCCC", "yellow": "#C19C00", "brightBlack": "#767676", "brightBlue": "#3B78FF", "brightCyan": "#61D6D6", "brightGreen": "#16C60C", "brightMagenta": "#B4009E", "brightRed": "#E74856", "brightWhite": "#F2F2F2", "brightYellow": "#F9F1A5" }
Set WindowBar "Colorful"
Set BorderRadius 8

# Sandbox initialization with sample manifest (hidden from recording)
Hide
Type "export PS1='perihelion:~$ ' && cd $HOME && rm -rf ~/.cache/perihelion-helion-demo && mkdir -p ~/.cache/perihelion-helion-demo && cd ~/.cache/perihelion-helion-demo"
Enter
Type "cat << 'EOF' > peri.toml"
Enter
Type "[package]"
Enter
Type "name = 'night-city-overhaul'"
Enter
Type "author = 'V'"
Enter
Type "version = '1.0.0'"
Enter
Type "priority = 'default'"
Enter
Type "description = 'Curated Cyberpunk 2077 mod setup'"
Enter
Type ""
Enter
Type "[target]"
Enter
Type "game_id = 'cyberpunk_2077'"
Enter
Type ""
Enter
Type "[dependencies]"
Enter
Type "'cyber_engine_tweaks' = '>= 1.25.0'"
Enter
Type "'redscript' = '>= 0.5.15'"
Enter
Type "'native_settings_ui' = '>= 1.9.0'"
Enter
Type "'weather_overhaul_hd' = '>= 2.1.0'"
Enter
Type ""
Enter
Type "[overrides]"
Enter
Type "'weather_overhaul_hd' = { priority = 'high' }"
Enter
Type "EOF"
Enter
Type "clear"
Enter
Sleep 1s
Show

# 1. Launch Helion TUI in workspace
Type "helion --no-recovery"
Sleep 500ms
Enter
Sleep 3s

# 2. Cycle to Mods view
Tab
Sleep 2s

# 3. Navigate mod entries
Down 2
Sleep 800ms
Up 1
Sleep 800ms

# 4. Toggle Inspector drawer
Type "i"
Sleep 2s
Type "i"
Sleep 800ms

# 5. Cycle to Conflicts view (3-tier resolution waterfall)
Tab
Sleep 2.5s

# 6. Cycle to Downloads view
Tab
Sleep 2s

# 7. Jump back to Overview tab
Type "1"
Sleep 2s

# 8. Real-time theme picker from Overview input
Type "/"
Sleep 500ms
Type "theme "
Sleep 1.5s
Down 2
Sleep 800ms
Down 1
Sleep 800ms
Enter
Sleep 2.5s

# 9. Quit Helion
Type "q"
Sleep 1s

# Cleanup workspace (hidden from recording)
Hide
Type "rm -rf ~/.cache/perihelion-helion-demo"
Enter
Sleep 500ms

