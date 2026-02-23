class Reef < Formula
  desc "Bash compatibility layer for fish shell"
  homepage "https://github.com/ZStud/reef"
  url "https://github.com/ZStud/reef/archive/v0.3.0.tar.gz"
  sha256 "PLACEHOLDER"
  license "MIT"

  depends_on "rust" => :build
  depends_on "fish"

  def install
    system "cargo", "install", *std_cargo_args

    # Fish functions → vendor_functions.d
    fish_vendor_functions = share/"fish/vendor_functions.d"
    fish_vendor_functions.install "fish/functions/export.fish"
    fish_vendor_functions.install "fish/functions/unset.fish"
    fish_vendor_functions.install "fish/functions/declare.fish"
    fish_vendor_functions.install "fish/functions/local.fish"
    fish_vendor_functions.install "fish/functions/readonly.fish"
    fish_vendor_functions.install "fish/functions/shopt.fish"
    fish_vendor_functions.install "fish/functions/fish_command_not_found.fish"

    # conf.d → vendor_conf.d
    fish_vendor_conf = share/"fish/vendor_conf.d"
    fish_vendor_conf.install "fish/conf.d/reef.fish"
  end

  test do
    assert_match "reef #{version}", shell_output("#{bin}/reef --version")
    # Detection: export is bash syntax
    system bin/"reef", "detect", "--", "export FOO=bar"
    # Translation: export → set -gx
    assert_equal "set -gx FOO bar", shell_output("#{bin}/reef translate -- 'export FOO=bar'").strip
  end
end
