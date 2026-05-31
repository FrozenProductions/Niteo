class Niteo < Formula
  desc "Standalone Rust CLI for TypeScript structural linting"
  homepage "https://github.com/FrozenProductions/Niteo"
  url "https://github.com/FrozenProductions/Niteo/archive/refs/tags/v0.1.3.tar.gz"
  sha256 "25d97539b7565c4198f3ae28e176d3c3c23908e1e8a0b3f878e09fa898d740c5"
  license "MIT"

  bottle do
    sha256 cellar: :any_skip_relocation, arm64_sequoia: "b2a54dd9495d854b34e42fe024b11daf7721f083be3cc13742d1e3ceb8abc64b"
    sha256 cellar: :any_skip_relocation, sequoia:       "a6a2e178dcfccf95cb5ab388afbaa8cdffeb297df62075487a967994c156ff71"
    sha256 cellar: :any_skip_relocation, x86_64_linux:  "a61ea451be5da0af16b8ba081c3157f581ab6cd7a85ec22b7dc3fffd29efe6ac"
  end

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Usage", shell_output("#{bin}/niteo --help")
  end
end
