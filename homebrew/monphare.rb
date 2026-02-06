# typed: false
# frozen_string_literal: true

class Monphare < Formula
  desc "Terraform/OpenTofu module constraint analyzer and dependency mapper"
  homepage "https://github.com/tanguc/monphare"
  version "0.1.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/tanguc/monphare/releases/download/v#{version}/monphare-darwin-aarch64.tar.gz"
      sha256 "PLACEHOLDER_DARWIN_ARM64"
    else
      url "https://github.com/tanguc/monphare/releases/download/v#{version}/monphare-darwin-x86_64.tar.gz"
      sha256 "PLACEHOLDER_DARWIN_X86_64"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/tanguc/monphare/releases/download/v#{version}/monphare-linux-aarch64.tar.gz"
      sha256 "PLACEHOLDER_LINUX_ARM64"
    else
      url "https://github.com/tanguc/monphare/releases/download/v#{version}/monphare-linux-x86_64.tar.gz"
      sha256 "PLACEHOLDER_LINUX_X86_64"
    end
  end

  def install
    bin.install "monphare"
  end

  test do
    system "#{bin}/monphare", "--version"
  end
end
