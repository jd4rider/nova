class Nova < Formula
  desc "Statically typed native language with an LLVM backend and Rust toolchain"
  homepage "https://github.com/jd4rider/nova"
  url "https://github.com/jd4rider/nova/releases/download/v0.1.0/nova-0.1.0-aarch64-apple-darwin.tar.gz"
  sha256 "03ad1c66644b90837ce5b44fb9fbb375ae7164d4be28851d041bdea0f367c067"
  version "0.1.0"
  license :cannot_represent

  depends_on arch: :arm64
  depends_on macos: :big_sur

  def install
    bin.install "nova-0.1.0-aarch64-apple-darwin/nova"
  end

  def caveats
    <<~EOS
      Nova invokes the host Clang to link native programs.
      Install the Xcode Command Line Tools if `clang` is not already on PATH:
        xcode-select --install
    EOS
  end

  test do
    assert_match "nova 0.1.0", shell_output("#{bin}/nova --version")
  end
end
