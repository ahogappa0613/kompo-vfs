class KompoVfs < Formula
  desc ""
  homepage "https://github.com/ahogappa0613/kompo-vfs"
  url "https://github.com/ahogappa0613/kompo-vfs.git", using: :git, branch: "main"
  head "https://github.com/ahogappa0613/kompo-vfs.git", branch: "main"
  version "0.1.0"

  depends_on "rust" => :build

  def install
    system "cargo build --release"

    bin.install "target/release/kompo-cli"
    lib.install "target/release/libkompo.a"
  end

  test do
    system bin/"kompo-cli", "--version"
    system "file", lib/"libkompo.a"
  end
end
