#\!/usr/bin/env bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY="$PROJECT_DIR/target/debug/gunita"
FW="/usr/libexec/ApplicationFirewall/socketfilterfw"

if [ \! -f "$BINARY" ]; then
  echo "Error: Binary not found at $BINARY"
  echo "Run \"cargo build\" first."
  exit 1
fi

case "${1:-}" in
  on)
    echo "Exposing gunita to LAN..."
    sudo "$FW" --add "$BINARY"
    sudo "$FW" --unblockapp "$BINARY"
    echo "Done. gunita is now accessible on the network."
    ;;
  off)
    echo "Blocking gunita from LAN..."
    sudo "$FW" --blockapp "$BINARY"
    echo "Done. gunita is no longer accessible on the network."
    ;;
  status)
    sudo "$FW" --listapps | grep -A 1 gunita || echo "gunita is not in the firewall list."
    ;;
  *)
    echo "Usage: $(basename "$0") {on|off|status}"
    echo ""
    echo "  on     - Allow gunita through the firewall"
    echo "  off    - Block gunita in the firewall"
    echo "  status - Check current firewall state for gunita"
    exit 1
    ;;
esac
