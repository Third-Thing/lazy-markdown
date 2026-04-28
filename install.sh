cargo build --release

install -Dm755 target/release/lazy-markdown \
  ~/.local/bin/lazy-markdown

install -Dm644 pkg/lazy-markdown.desktop \
  ~/.local/share/applications/lazy-markdown.desktop

install -Dm644 pkg/lazy-markdown-icon-256.png \
  ~/.local/share/icons/hicolor/256x256/apps/lazy-markdown.png

update-desktop-database ~/.local/share/applications
gtk-update-icon-cache ~/.local/share/icons/hicolor
