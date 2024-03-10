class KompoVfs < Formula
  desc ""
  homepage "https://github.com/ahogappa0613/kompo-vfs"
  url "https://github.com/ahogappa0613/kompo-vfs.git", tag: "v0.5.2", using: :git
  head "https://github.com/ahogappa0613/kompo-vfs.git", branch: "main"
  # license "Apache-2.0" => { with: "LLVM-exception" }

  depends_on "rust" => :build

  def install
    system "cargo", "install",
           "--bin", "kompo-cli",
           "--path", "./kompo-cli",
           "--root", prefix
  end

  test do
    system bin/"kompo-cli", "--version"
  end
end
