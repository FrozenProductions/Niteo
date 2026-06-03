class Niteo < Formula
  desc "Standalone Rust CLI for TypeScript structural linting"
  homepage "https://github.com/FrozenProductions/Niteo"
  url "https://github.com/FrozenProductions/Niteo/archive/refs/tags/v0.2.1.tar.gz"
  sha256 "bc3d2de4fbede9f7349da03bd4c4f24053394ba01d330ee0a027dec10b26a9e7"
  license "MIT"

  bottle do
    root_url "https://github.com/FrozenProductions/Niteo/releases/download/v0.2.1"
    sha256 cellar: :any_skip_relocation, arm64_sequoia: "7d7ff77453bd394d652a646603a0cc332a1733c5d2a5470363166d6e9040a992"
    sha256 cellar: :any_skip_relocation, sequoia:       "341f7119f137f822a01e2cf3329e3d0292a627adf6b6c5ca8aba45ec7f9a796d"
    sha256 cellar: :any,                 x86_64_linux:  "10ea04f0d6b03bc4100226639205c0390dcb3f3905ec3306ef7bbde20031d3f9"
  end






  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Usage", shell_output("#{bin}/niteo --help")
  end
end
