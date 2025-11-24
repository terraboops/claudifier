class Boopifier < Formula
  desc "Universal notification handler for Claude Code events"
  homepage "https://github.com/terraboops/boopifier"
  url "https://github.com/terraboops/boopifier/archive/v0.2.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "Apache-2.0"
  head "https://github.com/terraboops/boopifier.git", branch: "main"

  depends_on "rust" => :build
  depends_on "pkg-config" => :build
  depends_on "openssl@3"
  depends_on "alsa-lib" if OS.linux?

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Test that boopifier runs and shows help
    assert_match "Universal notification handler", shell_output("#{bin}/boopifier --help")
  end

  def caveats
    <<~EOS
      Boopifier requires configuration in ~/.claude/boopifier.json
      See documentation at: https://github.com/terraboops/boopifier

      Optional dependencies for full functionality:
      - notify-rust: Desktop notifications (works on macOS Notification Center)
      - rodio: Sound playback (CoreAudio on macOS)
      - signal-cli: Signal messenger integration (brew install signal-cli)

      Example usage:
        echo '{"hook_event_name": "Notification", "message": "test"}' | boopifier
    EOS
  end
end
