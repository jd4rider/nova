cask "nova-studio" do
  version "0.1.0"
  sha256 "4a89c3fed642b1f6e80474cc6f60970ed606e9dacae8c0f5ee99ef9c8c7dae91"

  url "https://github.com/jd4rider/nova/releases/download/v#{version}/Nova-Studio-#{version}-arm64.dmg"
  name "Nova Studio"
  desc "Desktop IDE for the Nova language"
  homepage "https://github.com/jd4rider/nova"

  depends_on arch: :arm64
  depends_on macos: :big_sur
  depends_on formula: "jd4rider/tap/nova"

  app "Nova Studio.app"

  caveats <<~EOS
    The Nova language (`nova` CLI) is installed automatically from jd4rider/tap.
    Nova Studio is unsigned and not notarized. If macOS blocks the first launch,
    right-click the app and choose Open, or run:
      xattr -dr com.apple.quarantine "/Applications/Nova Studio.app"
  EOS

  zap trash: [
    "~/Library/Application Support/Nova Studio",
    "~/Library/Preferences/dev.nova.studio.plist",
    "~/Library/Saved Application State/dev.nova.studio.savedState",
  ]
end
