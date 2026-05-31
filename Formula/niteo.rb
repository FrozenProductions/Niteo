class Niteo < Formula
  desc "Standalone Rust CLI for TypeScript structural linting"
  homepage "https://github.com/FrozenProductions/Niteo"
  url "https://github.com/FrozenProductions/Niteo/archive/refs/tags/v0.1.2.tar.gz"
  sha256 "b4e458c458d5fddfd57d8906003f0a1c258fdf20921e7d2b61ac43a0ffa6b291"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Usage", shell_output("#{bin}/niteo --help")
  end
end
