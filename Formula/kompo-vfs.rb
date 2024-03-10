class KompoVfs < Formula
  desc ""
  homepage "https://github.com/ahogappa0613/kompo-vfs"
  url "https://github.com/ahogappa0613/kompo-vfs.git", using: :git
  head "https://github.com/ahogappa0613/kompo-vfs.git", branch: "main"

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
