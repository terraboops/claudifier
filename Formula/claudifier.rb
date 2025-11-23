class Claudifier < Formula
  desc "Universal notification handler for Claude Code events"
  homepage "https://github.com/terraboops/claudifier"
  url "https://github.com/terraboops/claudifier/archive/v0.1.0.tar.gz"
  sha256 "f456463037c16b50c919a996f9362714549231272281fcc58da56818d9491e7f"
  license "Apache-2.0"
  head "https://github.com/terraboops/claudifier.git", branch: "main"

  depends_on "rust" => :build
  depends_on "pkg-config" => :build
  depends_on "openssl@3"
  depends_on "alsa-lib" if OS.linux?

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Test that claudifier runs and shows help
    assert_match "Universal notification handler", shell_output("#{bin}/claudifier --help")
  end

  def caveats
    <<~EOS
      Claudifier requires configuration in ~/.claude/claudifier.json
      See documentation at: https://github.com/terraboops/claudifier

      Optional dependencies for full functionality:
      - notify-rust: Desktop notifications (works on macOS Notification Center)
      - rodio: Sound playback (CoreAudio on macOS)
      - signal-cli: Signal messenger integration (brew install signal-cli)

      Example usage:
        echo '{"hook_event_name": "Notification", "message": "test"}' | claudifier
    EOS
  end
end
