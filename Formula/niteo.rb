class Niteo < Formula
  desc "Standalone Rust CLI for TypeScript structural linting"
  homepage "https://github.com/FrozenProductions/Niteo"
  url "https://github.com/FrozenProductions/Niteo/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "b390995840bb9dc7a0393d6d48566cf12bb9d0604aea1e364f4f9c5a5af4b0c6"
  license "MIT"

  bottle do
    root_url "https://github.com/FrozenProductions/Niteo/releases/download/v0.2.0"
    sha256 cellar: :any_skip_relocation, arm64_sequoia: "4a557c41d365ba7517b054c5b85b72c879fd232986d198cd6ba46f252c3a731a"
    sha256 cellar: :any_skip_relocation, sequoia:       "d80fe4105afecaf699eb2b0b4a1229a7547a11beb6e3f2ef94f2286d3cd5ef83"
    sha256 cellar: :any_skip_relocation, x86_64_linux:  "d6904abb4424012ca60f9e8f2a781699a7c69e4751c7ed4a6468f60fdbbef02d"
  end


  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Usage", shell_output("#{bin}/niteo --help")
  end
end
