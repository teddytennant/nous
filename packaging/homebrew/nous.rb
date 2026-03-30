class Nous < Formula
  desc "Decentralized everything-app — identity, messaging, governance, payments, AI, and more"
  homepage "https://github.com/teddytennant/nous"
  license "MIT"
  version "0.1.0"

  on_macos do
    on_arm do
      url "https://github.com/teddytennant/nous/releases/download/v#{version}/nous-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/teddytennant/nous/releases/download/v#{version}/nous-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/teddytennant/nous/releases/download/v#{version}/nous-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/teddytennant/nous/releases/download/v#{version}/nous-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "nous"
    bin.install "nous-api"
  end

  test do
    assert_match "nous", shell_output("#{bin}/nous --help")
  end
end
