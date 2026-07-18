# frozen_string_literal: true

# bmx - Homebrew Formula
# Install: brew install amkisko/tap/bmx
# Or: brew tap amkisko/tap && brew install bmx
class Bmx < Formula
  desc "Command-line tool that installs, builds, and runs software from source repositories"
  homepage "https://github.com/amkisko/bmx.rs"
  url "https://github.com/amkisko/bmx.rs/archive/refs/tags/v0.1.3.tar.gz"
  # Fill before release: shasum -a 256 <(curl -sL https://github.com/amkisko/bmx.rs/archive/refs/tags/vX.Y.Z.tar.gz)
  sha256 ""
  license "MIT"
  head "https://github.com/amkisko/bmx.rs.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "bmx #{version}", shell_output("#{bin}/bmx --version")
  end
end
