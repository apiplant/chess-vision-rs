# Generated from packaging/homebrew/chess-vision-rs.rb in apiplant/chess-vision-rs by the
# release workflow, which fills in the version and checksums and commits the
# result to apiplant/homebrew-tap as Formula/chess-vision-rs.rb. Changes belong in
# the source repository: the next release overwrites this file.
class ChessVisionRs < Formula
  desc "Recognise a chess position from an image and print FEN, Lichess links or a rendered board"
  homepage "https://github.com/apiplant/chess-vision-rs"
  version "@VERSION@"
  license "AGPL-3.0-only"

  # No bottles: the release archives *are* the binaries, so the formula only
  # unpacks what the tagged workflow already built for each platform.
  on_macos do
    on_arm do
      url "https://github.com/apiplant/chess-vision-rs/releases/download/v@VERSION@/chess-vision-rs-v@VERSION@-aarch64-apple-darwin.tar.gz"
      sha256 "@SHA_MACOS_ARM64@"
    end
  end
  on_linux do
    on_intel do
      url "https://github.com/apiplant/chess-vision-rs/releases/download/v@VERSION@/chess-vision-rs-v@VERSION@-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "@SHA_LINUX_X86_64@"
    end
    on_arm do
      url "https://github.com/apiplant/chess-vision-rs/releases/download/v@VERSION@/chess-vision-rs-v@VERSION@-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "@SHA_LINUX_ARM64@"
    end
  end

  def install
    bin.install "chess-vision"
    doc.install "README.md"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/chess-vision --version")
  end
end
