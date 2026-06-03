class Niteo < Formula
  desc "Standalone Rust CLI for TypeScript structural linting"
  homepage "https://github.com/FrozenProductions/Niteo"
  url "https://github.com/FrozenProductions/Niteo/archive/refs/tags/v0.2.1.tar.gz"
  sha256 "bc3d2de4fbede9f7349da03bd4c4f24053394ba01d330ee0a027dec10b26a9e7"
  license "MIT"

  bottle do
    root_url "https://github.com/FrozenProductions/Niteo/releases/download/v0.2.1"
    sha256 cellar: :any_skip_relocation, arm64_sequoia: "a87af7648cac0375b80f05bd6c670c5d110a25acf1f78d5974ae5d9f0915c1f6"
    sha256 cellar: :any_skip_relocation, sequoia:       "63bfd07a0f5725cc8fbf0fd4f386a05b6e9b7b190a8b77eaf1ced84c40cf82b6"
    sha256 cellar: :any,                 x86_64_linux:  "b5983aafb23fe823989a755b994d53bc25891f7e6ca841ac1975a98701062e11"
  end





  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Usage", shell_output("#{bin}/niteo --help")
  end
end
